//! 像素网格与绘制规则：每格一个键（ASCII 字节），`.` 透明。
//!
//! 模板只画剪影与细节，描边和明暗由这里按统一规则生成，所以 64 种外形风格一致：
//! - 描边：透明且与填充 4 邻接的像素成为描边 `o`；细节像素（胡须、闪光、字形）不产生描边。
//! - 明暗：光从左上来。填充区域左上边缘提亮、右下边缘压暗，一像素宽的地方保持原色。
//! - 分层：背后的部件、身体、身前的部件各自描边打光后再叠加，部件与身体之间因此有一道描边。
use alloc::vec;
use alloc::vec::Vec;

pub(crate) const CLEAR: u8 = b'.';

/// 不产生描边的细节键。
const DETAIL: &[u8] = b"WZx*!jGzR";

/// 可打光的填充键：(键, 亮部, 暗部)。
const SHADE: [(u8, u8, u8); 11] = [
    (b'b', b'h', b's'),
    (b'a', b'c', b'A'),
    (b'p', b'r', b'q'),
    (b'l', b'l', b'L'),
    (b't', b'i', b'T'),
    (b'g', b'G', b'g'),
    // 装扮的材质：布、皮革、帽子、叶子
    (b'C', b'E', b'D'),
    (b'B', b'H', b'J'),
    (b'M', b'N', b'O'),
    (b'V', b'I', b'K'),
    // 个体花纹
    (b'1', b'2', b'3'),
];

#[derive(Clone)]
pub(crate) struct Grid {
    pub w: i32,
    pub h: i32,
    pub cells: Vec<u8>,
}

impl Grid {
    pub fn new(w: i32, h: i32) -> Grid {
        Grid {
            w: w.max(0),
            h: h.max(0),
            cells: vec![CLEAR; (w.max(0) * h.max(0)) as usize],
        }
    }

    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            CLEAR
        } else {
            self.cells[(y * self.w + x) as usize]
        }
    }

    pub fn set(&mut self, x: i32, y: i32, k: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.cells[(y * self.w + x) as usize] = k;
        }
    }

    /// 把模板画到 (x, y)，`.` 不覆盖。
    pub fn blit(&mut self, x: i32, y: i32, rows: &[&str]) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, k) in row.bytes().enumerate() {
                if k != CLEAR {
                    self.set(x + dx as i32, y + dy as i32, k);
                }
            }
        }
    }

    /// 同上，但左右翻转。
    pub fn blit_flipped(&mut self, x: i32, y: i32, rows: &[&str]) {
        for (dy, row) in rows.iter().enumerate() {
            let n = row.len() as i32;
            for (dx, k) in row.bytes().enumerate() {
                if k != CLEAR {
                    self.set(x + n - 1 - dx as i32, y + dy as i32, k);
                }
            }
        }
    }

    /// 只画左半边的模板，镜像成左右对称的整体。
    pub fn blit_mirrored(&mut self, x: i32, y: i32, half: &[&str]) {
        let n = half.first().map_or(0, |r| r.len()) as i32;
        self.blit(x, y, half);
        self.blit_flipped(x + n, y, half);
    }

    /// 另一张同尺寸网格中的非透明像素盖到本网格上。
    pub fn over(&mut self, top: &Grid) {
        for (i, &k) in top.cells.iter().enumerate() {
            if k != CLEAR {
                self.cells[i] = k;
            }
        }
    }

    /// 自动描边与明暗，见模块说明。
    pub fn outline_and_shade(&mut self) {
        let src = self.clone();
        let solid = |x: i32, y: i32| {
            let k = src.get(x, y);
            k != CLEAR && !DETAIL.contains(&k)
        };
        for y in 0..self.h {
            for x in 0..self.w {
                if src.get(x, y) == CLEAR
                    && (solid(x + 1, y) || solid(x - 1, y) || solid(x, y + 1) || solid(x, y - 1))
                {
                    self.set(x, y, b'o');
                }
            }
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let k = src.get(x, y);
                let Some(&(_, light, dark)) = SHADE.iter().find(|(f, _, _)| *f == k) else {
                    continue;
                };
                let lit = src.get(x, y - 1) != k || src.get(x - 1, y) != k;
                let shaded = src.get(x, y + 1) != k || src.get(x + 1, y) != k;
                if lit && !shaded {
                    self.set(x, y, light);
                } else if shaded && !lit {
                    self.set(x, y, dark);
                }
            }
        }
    }

    /// 把另一张网格的非透明像素贴到 (x, y)。
    pub fn paste(&mut self, top: &Grid, x: i32, y: i32) {
        for ty in 0..top.h {
            for tx in 0..top.w {
                let k = top.get(tx, ty);
                if k != CLEAR {
                    self.set(x + tx, y + ty, k);
                }
            }
        }
    }

    /// 非透明像素的包围盒：(左, 上, 右, 下)，含边界。
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        for y in 0..self.h {
            for x in 0..self.w {
                if self.get(x, y) != CLEAR {
                    b = Some(match b {
                        None => (x, y, x, y),
                        Some((l, t, r, bt)) => (l.min(x), t.min(y), r.max(x), bt.max(y)),
                    });
                }
            }
        }
        b
    }

    pub fn find(&self, k: u8) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.h).flat_map(move |y| {
            (0..self.w).filter_map(move |x| (self.get(x, y) == k).then_some((x, y)))
        })
    }

    /// 平移：(dx, dy) 之外的部分丢弃。
    pub fn shifted(&self, dx: i32, dy: i32) -> Grid {
        let mut g = Grid::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                g.set(x + dx, y + dy, self.get(x, y));
            }
        }
        g
    }
}
