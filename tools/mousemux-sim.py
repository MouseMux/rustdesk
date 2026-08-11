#!/usr/bin/env python3
"""
mousemux-sim.py - impersonate MouseMux to exercise RustDesk's protocol side.

Creates a window with the same class AND title MouseMux uses, decodes everything
RustDesk sends, and replies with the real protocol so RustDesk's handling can be
tested without MouseMux running.

    python tools/mousemux-sim.py [--class mousemux-v3.main.window.query]

WHAT IT CHECKS
  - RustDesk finds us at all. Both sides pass the name as class AND title, which
    is the MouseMux lookup contract. (A class-only search with a genuine NULL
    title does work; the trap is a binding that marshals NULL as "", which
    matches nothing. Pass the name twice and the question never arises.)
  - NOTIFY_STARTUP carries a sane version and a usable RustDesk HWND
  - REQUEST_CONNECTION announces a protocol version we accept (121..=123)
  - SET_CONNECTION_NAME decodes as UTF-32 LE, four messages per code point -
    this is the one that was silently broken: RustDesk used to send one message
    per character, so every name arrived garbled
  - REQUEST_IDS gets answered and RustDesk stores the assigned HWIDs

IMPORTANT: stop the real MouseMux first, or FindWindowEx may return its window
instead of ours and the simulator will sit idle. The script warns if it detects a
competing window.
"""
import ctypes
import ctypes.wintypes as w
import sys
import time

u32 = ctypes.WinDLL("user32", use_last_error=True)

WM_APP = 0x8000
R2M = {
    WM_APP + 10: "NOTIFY_STARTUP",
    WM_APP + 20: "NOTIFY_SHUTDOWN",
    WM_APP + 30: "REQUEST_CONNECTION",
    WM_APP + 40: "SET_CONNECTION_NAME",
    WM_APP + 50: "REQUEST_IDS",
    WM_APP + 60: "RELEASE_CONNECTION",
}
M2R_STARTUP_BROADCAST = WM_APP + 100
M2R_MOUSE_ID          = WM_APP + 110
M2R_KEYBOARD_ID       = WM_APP + 120
M2R_USER_ADD          = WM_APP + 160
M2R_USER_REMOVE       = WM_APP + 170

PROTO_MIN, PROTO_MAX = 121, 123
VERS_MIN, VERS_MAX   = 100, 999
HWID_BASE            = 6001

CLASS = "mousemux-v3.main.window.query"

WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_longlong, w.HWND, w.UINT, w.WPARAM, w.LPARAM)


class WNDCLASSEX(ctypes.Structure):
    _fields_ = [
        ("cbSize", w.UINT), ("style", w.UINT), ("lpfnWndProc", WNDPROC),
        ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int),
        ("hInstance", w.HINSTANCE), ("hIcon", w.HICON), ("hCursor", w.HANDLE),
        ("hbrBackground", w.HBRUSH), ("lpszMenuName", w.LPCWSTR),
        ("lpszClassName", w.LPCWSTR), ("hIconSm", w.HICON),
    ]


class State:
    def __init__(self):
        self.rustdesk_hwnd = 0
        self.conns = {}          # conn_id -> {"bytes":[], "utf32":0, "n":0, "name":"", "ms":0, "kb":0}
        self.next_hwid = HWID_BASE
        self.users = 0
        self.problems = []
        self.seen = set()

    def conn(self, cid):
        return self.conns.setdefault(
            cid, {"utf32": 0, "n": 0, "name": "", "ms": 0, "kb": 0, "raw": []}
        )

    def flag(self, msg):
        self.problems.append(msg)
        print("   !! %s" % msg)


S = State()


def post(hwnd, msg, wp, lp):
    ok = u32.PostMessageW(w.HWND(hwnd), msg, w.WPARAM(wp), w.LPARAM(lp))
    if not ok:
        S.flag("PostMessage failed (err %d) msg=WM_APP+%d" % (ctypes.get_last_error(), msg - WM_APP))
    return ok


def on_message(hwnd, msg, wp, lp):
    name = R2M.get(msg)
    if name is None:
        return
    S.seen.add(name)

    if name == "NOTIFY_STARTUP":
        S.rustdesk_hwnd = lp
        ok = VERS_MIN <= wp <= VERS_MAX
        print("<- NOTIFY_STARTUP    version=%d  rustdesk_hwnd=0x%X %s"
              % (wp, lp, "" if ok else "  <-- OUT OF RANGE"))
        if not ok:
            S.flag("version %d outside %d..%d" % (wp, VERS_MIN, VERS_MAX))
        if lp == 0:
            S.flag("NOTIFY_STARTUP carried a NULL RustDesk HWND")

    elif name == "NOTIFY_SHUTDOWN":
        print("<- NOTIFY_SHUTDOWN   version=%d  hwnd=0x%X" % (wp, lp))

    elif name == "REQUEST_CONNECTION":
        c = S.conn(wp)
        c.update({"utf32": 0, "n": 0, "name": "", "raw": []})
        ok = PROTO_MIN <= lp <= PROTO_MAX
        print("<- REQUEST_CONNECTION conn=%d protocol=%d %s"
              % (wp, lp, "" if ok else "  <-- REJECTED, outside %d..%d" % (PROTO_MIN, PROTO_MAX)))
        if not ok:
            S.flag("protocol %d outside %d..%d - MouseMux would reject this connection"
                   % (lp, PROTO_MIN, PROTO_MAX))

    elif name == "SET_CONNECTION_NAME":
        c = S.conn(wp)
        byte = lp & 0xFF
        if not (0 <= lp <= 255):
            S.flag("name payload %d is not a byte (0..255)" % lp)
        c["raw"].append(lp)
        # exactly what rustdesk_handler.c does
        c["utf32"] |= (byte & 0xFF) << (c["n"] * 8)
        c["n"] += 1
        if c["n"] == 4:
            cp = c["utf32"]
            if cp == 0:
                print("<- SET_CONNECTION_NAME conn=%d  [terminator]  name=%r" % (wp, c["name"]))
                c["done"] = True
            else:
                try:
                    c["name"] += chr(cp)
                except ValueError:
                    S.flag("code point U+%04X is not representable" % cp)
            c["utf32"] = 0
            c["n"] = 0

    elif name == "REQUEST_IDS":
        c = S.conn(wp)
        if c["n"] != 0:
            S.flag("REQUEST_IDS arrived mid-character (%d/4 bytes buffered) - "
                   "the name stream is not 4-byte aligned" % c["n"])
        if not c.get("done"):
            S.flag("REQUEST_IDS arrived without a 4-byte null terminator")
        target = lp or S.rustdesk_hwnd
        c["ms"] = S.next_hwid; S.next_hwid += 1
        c["kb"] = S.next_hwid; S.next_hwid += 1
        print("<- REQUEST_IDS       conn=%d  name=%r" % (wp, c["name"]))
        print("   -> MOUSE_ID    conn=%d id=%d (0x%X)" % (wp, c["ms"], c["ms"]))
        post(target, M2R_MOUSE_ID, wp, c["ms"])
        print("   -> KEYBOARD_ID conn=%d id=%d (0x%X)" % (wp, c["kb"], c["kb"]))
        post(target, M2R_KEYBOARD_ID, wp, c["kb"])
        S.users += 1
        print("   -> USER_ADD    user=%d total=%d" % (wp, S.users))
        post(target, M2R_USER_ADD, wp, S.users)

    elif name == "RELEASE_CONNECTION":
        c = S.conns.pop(wp, None)
        S.users = max(0, S.users - 1)
        print("<- RELEASE_CONNECTION conn=%d  (name was %r)" % (wp, (c or {}).get("name")))
        if S.rustdesk_hwnd:
            post(S.rustdesk_hwnd, M2R_USER_REMOVE, wp, S.users)


def wndproc(hwnd, msg, wp, lp):
    try:
        on_message(hwnd, msg, wp, lp)
    except Exception as e:                      # never let an exception escape into Windows
        print("   !! handler error: %r" % e)
    return u32.DefWindowProcW(w.HWND(hwnd), msg, w.WPARAM(wp), w.LPARAM(lp))


def main():
    global CLASS
    if "--class" in sys.argv:
        CLASS = sys.argv[sys.argv.index("--class") + 1]

    existing = u32.FindWindowExW(None, None, CLASS, CLASS)
    if existing:
        print("!! A window with class+title %r already exists (hwnd 0x%X)." % (CLASS, existing))
        print("!! That is probably the real MouseMux. RustDesk may talk to it instead of")
        print("!! this simulator. Stop MouseMux first for an unambiguous test.")
        print()

    proc = WNDPROC(wndproc)                     # keep a reference or it is collected
    wc = WNDCLASSEX()
    wc.cbSize = ctypes.sizeof(WNDCLASSEX)
    wc.lpfnWndProc = proc
    wc.lpszClassName = CLASS
    wc.hInstance = None
    if not u32.RegisterClassExW(ctypes.byref(wc)):
        err = ctypes.get_last_error()
        if err != 1410:                         # ERROR_CLASS_ALREADY_EXISTS
            print("RegisterClassEx failed: %d" % err); return 1

    # class AND title identical, exactly as MouseMux and RustDesk both do
    hwnd = u32.CreateWindowExW(0, CLASS, CLASS, 0, 0, 0, 0, 0, None, None, None, None)
    if not hwnd:
        print("CreateWindowEx failed: %d" % ctypes.get_last_error()); return 1

    print("=" * 70)
    print(" MouseMux simulator listening")
    print("   class/title : %s" % CLASS)
    print("   hwnd        : 0x%X" % hwnd)
    print("   accepts     : protocol %d..%d, version %d..%d" % (PROTO_MIN, PROTO_MAX, VERS_MIN, VERS_MAX))
    print("   Ctrl+C to stop and print a summary")
    print("=" * 70)

    # If RustDesk is already up it will not re-notify on its own; nudge it the way
    # MouseMux does at startup.
    rd = u32.FindWindowExW(None, None, "mousemux-v3.rustdesk.window.query",
                                       "mousemux-v3.rustdesk.window.query")
    if rd:
        print("-> RustDesk already running (hwnd 0x%X), sending STARTUP_BROADCAST" % rd)
        post(rd, M2R_STARTUP_BROADCAST, 0, 0)
    else:
        print("   (RustDesk not running yet - start it and it will find us)")
    print()

    msg = w.MSG()
    try:
        while u32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
            u32.TranslateMessage(ctypes.byref(msg))
            u32.DispatchMessageW(ctypes.byref(msg))
    except KeyboardInterrupt:
        pass

    print()
    print("=" * 70)
    print(" SUMMARY")
    print("   messages seen : %s" % (", ".join(sorted(S.seen)) or "NONE"))
    for cid, c in S.conns.items():
        print("   conn %-4d name=%r mouse=%s keyboard=%s" % (cid, c["name"], c["ms"], c["kb"]))
    if S.problems:
        print("   PROBLEMS:")
        for p in S.problems:
            print("     - %s" % p)
    else:
        print("   no protocol problems detected")
    print("=" * 70)
    return 1 if S.problems else 0


if __name__ == "__main__":
    sys.exit(main())
