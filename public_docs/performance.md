# Performance settings

The **Performance** page decides how much of your computer's memory Kooka
Pictura may use, how many undo steps it keeps, how it caches the picture for
fast drawing, where it puts image data that does not fit in memory, and
whether it uses your graphics card.

Open it with **Edit ▸ Preferences ▸ Performance**.

Every change on this page **takes effect immediately**. You do not need to
restart Kooka Pictura, which is different from Photoshop. Your choices are
saved straight away and come back the next time you start. They are also
included when you [back up your settings](settings-backup.md).

## Memory Usage

**Memory Usage** is the share of your computer's memory (RAM) that Kooka
Pictura may use, from 5 % to 100 %. The default is 60 %.

Drag the slider or type a number; the two move together. The line below,
**Let Kooka Pictura Use**, shows the actual limit in megabytes. It is a little
less than the percentage of your RAM, because Kooka Pictura keeps a safety
margin. If your system already limits how much memory apps may use, for
example in a container or in some desktop sessions, the lower limit applies
and this line shows it.

### When you raise it

Kooka Pictura keeps more of your images in memory, so it reads from the
scratch disk less often. Larger edits, such as a filter on a very big image or
a brush stroke on a huge layer, now have room to run.

### When you lower it

- **Within a fraction of a second**, Kooka Pictura starts moving parts of your
  open images out of memory. It first compresses them in memory, then writes
  them to the [scratch disk](#scratch-disks) if more room is still needed. You
  keep working while this happens. Those parts are read back the next time you
  view or edit them, which can make that edit a little slower.
- **Edits that need more room than is left are refused.** Instead of letting
  your computer run out of memory, Kooka Pictura stops the edit and shows a
  message such as *"Could not paint: not enough memory"*. Your document is not
  changed. Raise Memory Usage, close other documents, or use **Edit ▸ Purge**
  to free memory, then try again.
- **Undo history** is trimmed at your next edit, not straight away: the
  oldest steps go first until history fits its share of the new limit.
- Documents that are already open keep the size of their drawing cache until
  you reopen them.

Some memory cannot be moved to disk: the program itself, its windows and
the graphics card's buffers. If the limit is very low, this alone can be more
than the limit, and most edits will be refused. Your system monitor may also
show Kooka Pictura's size falling more slowly than the limit, because freed
memory is not always handed back to the system at once.

**Tip:** 50 % to 70 % suits most computers. Set it too high and your whole
system can slow down as it runs out of memory for other programs. Set it too
low and you will see more "not enough memory" messages and more waiting on
the scratch disk.

## History & Cache

### History States

**History States** is how many steps the History panel keeps for each
document, from 1 to 1000. The default is 20. Each step is one thing you can
undo.

- **Raising it** lets you undo further back, starting with the edits you make
  from now on. Steps that were already dropped do not come back.
- **Lowering it** removes the oldest steps from **every open document
  immediately**, and the History panel updates. If you have undone all the way
  back to the oldest step, the newest steps you could redo are removed
  instead, so the picture you are looking at does not change.
- **Snapshots** in the History panel are never removed by this setting.
- Removed steps cannot be brought back.

More steps can use more memory, mostly for edits that change the whole image,
such as a filter. Small edits like a brush stroke cost little. However high you
set History States, Kooka Pictura also drops the oldest steps when history
uses up its share of the memory limit.

### Cache Levels

To draw quickly when you zoom out, Kooka Pictura keeps smaller copies of the
picture: half size, quarter size, eighth size and so on. **Cache Levels** is
how many sizes it keeps, counting the full-size picture, from 2 to 8. The
default is 8.

- A change applies to every open window the next time it redraws. There is
  nothing to wait for.
- **Fewer levels** use a little less memory, but zooming far out on a large
  image is slower, because Kooka Pictura has to shrink a bigger copy each time
  it draws. A very large brush on a large image can also feel less smooth,
  although the result is exactly the same.
- **More levels** keep zoomed-out views fast.
- Smaller images are not affected. A 4,000-pixel photo uses only four
  levels, so any setting from 4 up behaves the same for it.

Photoshop's default is lower. Kooka Pictura uses 8 so that large images stay
fast when zoomed out.

### Optimize Cache Levels for Documents that are

These three buttons are shortcuts for Cache Levels:

| Button | Sets Cache Levels to | Use it when |
|---|---|---|
| **Tall and Thin** | 4 | You mostly work on small images and want to keep memory free. |
| **Default** | 8 | Most work. |
| **Big and Flat** | 8 | You work on very large images. |

In Photoshop these buttons also change the cache tile size and adapt to your
computer. In Kooka Pictura they set Cache Levels only.

### Cache Tile Size

Kooka Pictura stores and draws images in squares of 256 × 256 pixels. **Cache
Tile Size** shows this size for reference. You cannot change it.

## Scratch Disks

When your images do not fit in the memory Kooka Pictura may use, it writes
parts of them to **scratch disks**: folders on your drives. The default is a
folder in your cache folder, usually `~/.cache/kooka-pictura/scratch`. While
you use only the default, the list shows it greyed out.

The list shows each folder with its free space. Kooka Pictura fills the
**first** folder, and moves on to the next only when that one is full.

- **Add…** adds a folder to the end of the list. The first time you add one,
  the default folder stays at the top, so you add space rather than replace
  it. You can have up to four folders; **Add…** is greyed out at four.
- **Remove** takes the selected folder off the list. Remove every folder and
  the default is used again.
- **Move Up** and **Move Down** change the order, and with it which folder is
  used first.

### What happens when you change the list

- The change applies **immediately** to new data. Kooka Pictura does not copy
  or move anything, so there is no waiting, and it does not freeze. If it is
  writing to a scratch disk at that moment, the change takes effect as soon as
  that write finishes.
- Data already on a folder you removed stays there and is still read from
  there until it is no longer needed. Leave that drive connected while Kooka
  Pictura is running.
- Scratch files have no name on disk and disappear when Kooka Pictura closes,
  even after a crash. You never need to clean them up.

### Choosing a good scratch disk

- Use a fast, local drive, ideally an SSD.
- Avoid network drives and removable drives that might be unplugged.
- A folder marked **(tmpfs: uses RAM)** lives in memory, not on a disk, so it
  does not help when memory is short. Choose a folder on a real drive.
- Keep plenty of free space. If every scratch disk is full, Kooka Pictura
  refuses the edit and says the scratch disks are full, and your document is
  left unchanged.

## Graphics Processor Settings

This section names the graphics card Kooka Pictura found, for example
*"Detected Graphics Processor: AMD Radeon 780M Graphics"*, or says *"No
graphics processor found"*.

**Use GPU Compute** lets Kooka Pictura use the graphics card to draw the
picture and to speed up some filters and brushes. The results are the same as
without it. The graphics card only makes the work faster.

- Turning it **off** applies to every open document immediately. Kooka
  Pictura then does everything on the processor (CPU). Try this if you see
  drawing problems on your graphics card.
- Turning it **on** again also makes Kooka Pictura try the graphics card again
  at the next start, even if it stopped using it after a graphics-driver
  crash.
- The checkbox is greyed out when there is no usable graphics card.

The status bar shows **GPU** or **CPU** to tell you which one is in use.

## Checking memory in the status bar

Two readouts at the bottom of the window show how memory is going. Click the
small **▾** button next to the document size and choose one:

- **Scratch Sizes**: two numbers such as `850.2M/2.04G`. The first is the
  memory Kooka Pictura is using now. The second is the most it may use, set by
  Memory Usage. `M` means megabytes and `G` gigabytes.
- **Efficiency**: how much of Kooka Pictura's recent work was spent working,
  rather than waiting for the scratch disk. **100%** means everything fitted
  in memory. If it often stays below about 95 %, raise Memory Usage, use a
  faster scratch disk, or lower History States.

Both update every two seconds while shown. Choose **Document Sizes** to
return to the usual readout.
