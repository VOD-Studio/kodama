"""真实 pty 验收 pet-cli live --account：画出账号宠物、按 f 喂食由服务确认、q 退出并恢复终端。

用法: python3 account_live.py <pet-cli>   （环境变量里已有登录凭据，见 tests/e2e/service-e2e.mjs）
"""
import os, pty, sys, time, select, termios, struct, fcntl

BIN = sys.argv[1]

def fresh_termios():
    m, s = os.openpty()
    try:
        return termios.tcgetattr(s)
    finally:
        os.close(m); os.close(s)

before = fresh_termios()
env = {**os.environ, "TERM": "xterm-256color", "LANG": "en_US.UTF-8"}
pid, fd = pty.fork()
if pid == 0:
    os.execve(BIN, [BIN, "live", "--account"], env)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))
buf = b""

def pump(sec, until=None):
    global buf
    end = time.time() + sec
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                d = os.read(fd, 65536)
            except OSError:
                return
            if not d:
                return
            buf += d
            if until and until in buf:
                return

pump(10, b"Lv")            # 读到投影后状态行出现等级
os.write(fd, b"f")
pump(10, b"feed: ok")      # 服务确认了这次喂食
os.write(fd, b"q")
pump(1)
status = None
for _ in range(60):
    p, st = os.waitpid(pid, os.WNOHANG)
    if p:
        status = st
        break
    time.sleep(0.05)
if status is None:
    os.kill(pid, 9)
after = termios.tcgetattr(fd) if status is not None else None

checks = [
    (b"Lv" in buf, "the account pet's status line appears"),
    (b"\xe2\x96\x80" in buf or b"\xe2\x96\x84" in buf, "the pet is drawn as half-block pixel art"),
    (b"feed: ok" in buf, "pressing f feeds the pet through the service"),
    (status is not None and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0, "q exits 0"),
    (after is not None and after[3] == before[3], "termios restored"),
    (b"\x1b[?1049l" in buf and b"\x1b[?25h" in buf, "screen and cursor restored"),
]
for ok, msg in checks:
    print(("PASS " if ok else "FAIL ") + msg)
sys.exit(0 if all(ok for ok, _ in checks) else 1)
