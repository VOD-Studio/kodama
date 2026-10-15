//! `live --account` 的数据源：后台线程读写服务，界面线程每帧只取最新结果，网络慢也不会卡住动画。
use super::client::{Client, Failure};
use kodama_core::Pet;
use kodama_term::outfit::Outfit;
use kodama_term::{Branch, Pose};
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Receiver, Sender};

/// 每隔多少个呈现刻度（100ms）重新读取一次投影。
const REFRESH_TICKS: u64 = 600;
/// 动作成功后对应姿态持续的刻度数。
const REACTION_TICKS: u64 = 30;

enum Req {
    Fetch,
    Act(&'static str),
}

enum Resp {
    View(Value),
    Acted(&'static str, Result<Value, Failure>),
    Failed(Failure),
}

/// 画出宠物与状态行需要的全部字段。
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub species: u8,
    pub genes: [u8; 8],
    /// 出生稀有度：决定光效。
    pub rarity: u8,
    pub lineage: Vec<Branch>,
    /// 身上的装扮。
    pub outfit: Outfit,
    pub phase: String,
    pub mood: String,
    pub food: u32,
    pub medicine: u32,
    pub level: u8,
    /// 正在探险时：主题与回来的逻辑时间（分钟）。
    pub away: Option<(String, u64)>,
    /// 本版本之前领取的个体没有外形记录。
    pub known_look: bool,
}

impl View {
    /// 由投影（`GET /v1/pets/me` 的 value）或动作结果（`Outcome`）构造；后者没有外形，沿用 `prev`。
    pub fn parse(v: &Value, prev: Option<&View>) -> Option<View> {
        let pet: Pet = serde_json::from_value(v["pet"].clone()).ok()?;
        let look = &v["appearance"];
        let (species, genes, known_look) = match (
            look["species"].as_u64(),
            serde_json::from_value::<[u8; 8]>(look["genes"].clone()),
        ) {
            (Some(s), Ok(g)) => (s as u8, g, true),
            _ => prev.map_or((0, [0; 8], false), |p| (p.species, p.genes, p.known_look)),
        };
        Some(View {
            species,
            genes,
            rarity: look["rarity"]
                .as_u64()
                .map_or(prev.map_or(0, |p| p.rarity), |r| r as u8),
            lineage: pet.growth.lineage.clone(),
            outfit: pet.wardrobe.worn.clone(),
            phase: v["phase"].as_str().unwrap_or("healthy").into(),
            mood: v["mood"].as_str().unwrap_or("").into(),
            food: pet.food,
            medicine: pet.medicine,
            level: pet.growth.level(),
            away: pet
                .travels
                .trip
                .as_ref()
                .map(|t| (t.topic.clone(), t.back_at)),
            known_look,
        })
    }

    /// 平时的姿态由生命阶段与情绪决定。
    pub fn pose(&self) -> Pose {
        match (self.phase.as_str(), self.mood.as_str()) {
            ("dead", _) => Pose::End,
            ("sick", _) => Pose::Sick,
            (_, "tired") => Pose::Sleep,
            (_, "hungry") => Pose::Observe,
            _ => Pose::Idle,
        }
    }
}

pub struct Watch {
    tx: Sender<Req>,
    rx: Receiver<Resp>,
    pub view: Option<View>,
    pub message: String,
    reaction: Option<(Pose, u64)>,
    last_fetch: u64,
}

impl Watch {
    /// 读取已保存的凭据并启动后台线程；没有登录时直接失败，不进入终端模式。
    pub fn start() -> Result<Self, Failure> {
        let mut client = Client::from_saved()?;
        let (tx, worker_rx) = channel::<Req>();
        let (worker_tx, rx) = channel::<Resp>();
        std::thread::spawn(move || {
            for req in worker_rx {
                let resp = match req {
                    Req::Fetch => match client.call("GET", "/v1/pets/me", None) {
                        Ok(v) => Resp::View(v),
                        Err(f) => Resp::Failed(f),
                    },
                    Req::Act(a) => Resp::Acted(a, super::act_with(&mut client, &json!(a))),
                };
                if worker_tx.send(resp).is_err() {
                    break;
                }
            }
        });
        let _ = tx.send(Req::Fetch);
        Ok(Self {
            tx,
            rx,
            view: None,
            message: "connecting…".into(),
            reaction: None,
            last_fetch: 0,
        })
    }

    /// 每帧调用：收下后台结果，并定期重新读取。
    pub fn tick(&mut self, t: u64) {
        while let Ok(resp) = self.rx.try_recv() {
            match resp {
                Resp::View(v) => {
                    self.view = View::parse(&v, self.view.as_ref());
                    if self.message == "connecting…" {
                        self.message = match &self.view {
                            Some(view) if !view.known_look => {
                                "this pet was adopted before appearances were recorded".into()
                            }
                            Some(_) => String::new(),
                            None => "unexpected response from the pet service".into(),
                        };
                    }
                }
                Resp::Acted(action, Ok(v)) => {
                    self.view = View::parse(&v, self.view.as_ref()).or(self.view.take());
                    self.message = format!("{action}: ok");
                    let pose = match action {
                        "feed" => Pose::Eat,
                        "play" => Pose::Play,
                        "rest" => Pose::Sleep,
                        _ => Pose::Observe,
                    };
                    self.reaction = Some((pose, t + REACTION_TICKS));
                }
                Resp::Acted(action, Err(f)) => {
                    self.message = format!("{action}: {}", f.code);
                    self.reaction = Some((Pose::Startled, t + REACTION_TICKS / 2));
                }
                Resp::Failed(f) => self.message = format!("{}: {}", f.code, f.message),
            }
        }
        if t >= self.last_fetch + REFRESH_TICKS {
            self.last_fetch = t;
            let _ = self.tx.send(Req::Fetch);
        }
    }

    pub fn act(&mut self, action: &'static str) {
        self.message = format!("{action}…");
        let _ = self.tx.send(Req::Act(action));
    }

    pub fn pose(&self, t: u64) -> Pose {
        match (self.reaction, &self.view) {
            (Some((p, until)), _) if t < until => p,
            (_, Some(v)) => v.pose(),
            (_, None) => Pose::Observe,
        }
    }

    pub fn status(&self) -> String {
        let pet = match &self.view {
            Some(v) => {
                let away = match &v.away {
                    // 逻辑时间是 Unix 分钟：换算成还要多久
                    Some((topic, back)) => {
                        let left = back.saturating_sub(super::client::now() / 60);
                        if left == 0 {
                            format!(" · back from {topic}, run `kodama act return`")
                        } else {
                            format!(
                                " · exploring {topic}, back in {}h{:02}m",
                                left / 60,
                                left % 60
                            )
                        }
                    }
                    None => String::new(),
                };
                format!(
                    "Lv{} food {} med {} {} {}{away}",
                    v.level, v.food, v.medicine, v.phase, v.mood
                )
            }
            None => String::new(),
        };
        format!(
            "q quit  f feed p play r rest h heal  {pet}  {}",
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection(phase: &str, mood: &str) -> Value {
        let pet = kodama_core::Pet::newborn("p".into(), 0);
        json!({"pet": pet, "phase": phase, "mood": mood, "version": 1,
               "appearance": {"species": 3, "genes": [1, 2, 3, 4, 5, 6, 7, 8], "rarity": 0}})
    }

    #[test]
    fn a_projection_becomes_a_drawable_view() {
        let v = View::parse(&projection("healthy", "happy"), None).unwrap();
        assert_eq!(
            (v.species, v.genes, v.food, v.level, v.known_look),
            (3, [1, 2, 3, 4, 5, 6, 7, 8], 5, 1, true)
        );
        assert_eq!(v.pose(), Pose::Idle);
    }

    #[test]
    fn an_action_outcome_keeps_the_known_appearance() {
        let first = View::parse(&projection("healthy", "happy"), None).unwrap();
        let mut outcome = projection("sick", "sick");
        outcome.as_object_mut().unwrap().remove("appearance");
        let next = View::parse(&outcome, Some(&first)).unwrap();
        assert_eq!((next.species, next.genes), (first.species, first.genes));
        assert_eq!(next.pose(), Pose::Sick);
    }

    #[test]
    fn life_phase_and_mood_choose_the_pose() {
        let pose = |p, m| View::parse(&projection(p, m), None).unwrap().pose();
        assert_eq!(pose("dead", "sick"), Pose::End);
        assert_eq!(pose("healthy", "tired"), Pose::Sleep);
        assert_eq!(pose("warned", "hungry"), Pose::Observe);
    }

    #[test]
    fn pets_without_a_recorded_appearance_still_draw() {
        let mut v = projection("healthy", "content");
        v["appearance"] = Value::Null;
        let view = View::parse(&v, None).unwrap();
        assert!(!view.known_look);
    }
}
