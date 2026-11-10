"""The memory benchmark's scenarios M1-M13 (issue #1210), for profile-tools.py.

Every later bounded-memory issue (#1209) names its savings by these IDs. Each
scenario is a setup (prerequisites, run before the peak reset and not measured)
and its measured steps, driven through the control server. Geometry is given at
17000x17000 and scaled by the document's side, so a 4250 run means the same.

Owner decisions: M3 fills the Background, so Layer 1 stays empty for M4-M6; M9
and M10 act on M4's square on Layer 1 with no selection.
"""

REFERENCE_SIDE = 17000


class Ctx:
    """What a step needs: the client, the document size, and file paths."""

    def __init__(self, client, w, h, tmp, fixture):
        self.client, self.w, self.h, self.tmp, self.fixture = client, w, h, tmp, fixture

    def px(self, reference):
        """`reference` pixels at 17000, scaled to this document (at least 1)."""
        return max(1, round(reference * min(self.w, self.h) / REFERENCE_SIDE))

    @property
    def centre(self):
        return self.w / 2, self.h / 2

    def call(self, method, **params):
        return self.client.call(method, **params)[0]

    def layer_path(self, name):
        tree = self.call("layer_tree")
        for layer in tree["layers"]:
            if layer["name"] == name:
                return layer["path"]
        raise RuntimeError(f"no layer named {name!r}")


def new_document(c):
    # File > New's document: the bottom layer is the locked white Background.
    c.call("document", op="new", width=c.w, height=c.h, background="white", background_layer=True)


def add_layer(c):
    c.call("layer_op", op="add", index=0)


def activate(c, name):
    c.call("set_active_layer", path=c.layer_path(name))


def marquee(c):
    side = c.px(1000)
    x, y = c.centre
    c.call("selection", op="rect", x=round(x - side / 2), y=round(y - side / 2), w=side, h=side)


def deselect(c):
    c.call("selection", op="deselect")


def fill_background(c):
    activate(c, "Background")
    # The default foreground, opaque black, as Edit > Fill's ARGB colour.
    c.call("run_command", id="edit.fill", params={"color": 0xFF000000})


def bucket(c, quarter=2):
    """A Paint Bucket click on Layer 1 at 1/quarter of each side (the centre)."""
    activate(c, "Layer 1")
    c.call("set_tool", tool="paintbucket")
    x, y = c.w / quarter, c.h / quarter
    c.call("pointer", op="down", space="image", x=x, y=y)
    c.call("pointer", op="up", space="image", x=x, y=y)


def brush_stroke(c):
    activate(c, "Layer 1")
    c.call("set_tool", tool="brush")
    c.call("tool_options", options={"brush": {"size": c.px(300), "hardness": 100, "opacity": 100, "flow": 100}})
    x, y = c.centre
    half = c.px(4000) / 2
    steps = 60
    c.call("pointer", op="down", space="image", x=x - half, y=y)
    for i in range(1, steps + 1):
        c.call("pointer", op="move", space="image", x=x - half + 2 * half * i / steps, y=y)
    c.call("pointer", op="up", space="image", x=x + half, y=y)


def undo_redo(c):
    for _ in range(3):
        c.call("edit", op="undo")
    for _ in range(3):
        c.call("edit", op="redo")


def snapshot_restore(c):
    c.call("history", op="snapshot", label="Benchmark")
    snapshots = c.call("history", op="list")["snapshots"]
    c.call("history", op="restore", index=len(snapshots) - 1)


def move_layer(c):
    activate(c, "Layer 1")
    c.call("set_tool", tool="move")
    x, y = c.centre
    dx = c.px(500)
    c.call("pointer", op="down", space="image", x=x, y=y)
    for i in range(1, 9):
        c.call("pointer", op="move", space="image", x=x + dx * i / 8, y=y)
    c.call("pointer", op="up", space="image", x=x + dx, y=y)


def free_transform(c):
    c.call("layer_op", op="free_transform", index=int(c.layer_path("Layer 1")), scale=150.0, angle=15.0)


def save_psd(c):
    c.call("document", op="save_as", path=f"{c.tmp}/benchmark-save.psd")


def open_fixture(c):
    c.call("document", op="open", path=c.fixture)


def pan_zoom(c):
    c.call("dispatch_command", id="view.fitOnScreen")
    c.call("status")
    c.call("dispatch_command", id="view.actualPixels")
    c.call("status")
    x, y = c.centre
    # Alt+wheel pans horizontally by 120 px a step, one way only (the control
    # wheel is always positive): the document's width, clamped at its edge.
    left = c.w
    while left > 0:
        steps = max(1, min(16, round(left / 120)))
        c.call("pointer", op="scroll", space="image", x=x, y=y, steps=steps, modifiers="alt")
        c.call("status")
        left -= steps * 120
    c.call("dispatch_command", id="view.fitOnScreen")


def bucket_in_marquee(c):
    marquee(c)
    bucket(c)


def m7_setup(c):
    new_document(c)
    add_layer(c)
    marquee(c)
    fill_background(c)
    bucket(c)
    deselect(c)
    # Outside the square: a black click on the black square changes nothing.
    bucket(c, quarter=4)
    brush_stroke(c)


# id -> (gesture, setup steps, measured steps, needs the M12 fixture)
SCENARIOS = {
    "M1": ("New document", [], [new_document], False),
    "M2": ("Add one empty layer", [new_document], [add_layer], False),
    "M3": ("Marquee 1000² at centre, Edit > Fill on Background",
           [new_document, add_layer, marquee], [fill_background], False),
    "M4": ("Paint Bucket in the marquee on Layer 1", [new_document, add_layer, marquee], [bucket], False),
    "M5": ("Paint Bucket on Layer 1, no selection", [new_document, add_layer], [bucket], False),
    "M6": ("Brush 300 px hard, 4000 px stroke", [new_document, add_layer], [brush_stroke], False),
    # #1215: twenty M6 strokes, so history's budget is exercised under --memory-max.
    "M6x20": ("Twenty M6 brush strokes", [new_document, add_layer], [brush_stroke] * 20, False),
    "M7": ("Undo x3, redo x3 after M3-M6", [m7_setup], [undo_redo], False),
    "M8": ("Named snapshot, then restore it", [new_document, add_layer, brush_stroke], [snapshot_restore], False),
    "M9": ("Move the square by 500 px", [new_document, add_layer, bucket_in_marquee, deselect], [move_layer], False),
    "M10": ("Free Transform the square 150 %, 15°", [new_document, add_layer, bucket_in_marquee, deselect],
            [free_transform], False),
    "M11": ("Save as PSD", [new_document, add_layer, bucket_in_marquee], [save_psd], False),
    "M12": ("Open the generated 4-layer PSD", [], [open_fixture], True),
    "M13": ("Fit, 100 %, pan across, fit", [new_document], [pan_zoom], False),
}
