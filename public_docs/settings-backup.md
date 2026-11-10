# Backing up and restoring settings

Kooka Pictura keeps your preferences, panel layout, workspaces, tool options,
keyboard shortcuts, swatches, action settings, adjustment presets and PDF
presets in its settings folders. You can copy all of them into one
`pictura.json` file and restore them later, on this computer or another one.

## Export your settings

1. Choose **Edit ▸ Preferences** and open the **General** page.
2. Click **Export Settings…**.
3. Choose where to save the file. The suggested name is `pictura.json`.
4. Click **Save**.

The file is plain JSON. It contains:

- every Preferences pane (theme, interface, performance, units, guides, type,
  file handling and the rest)
- the panel layout, window position and size
- your workspaces
- the options bar settings for every tool
- keyboard shortcuts you changed
- swatches, Actions settings and recent files
- adjustment presets, action sets and PDF presets

Machine-specific caches and crash-recovery snapshots are not included.

## Restore your settings

1. Choose **Edit ▸ Preferences** and open the **General** page.
2. Click **Import Settings…**.
3. Choose the `pictura.json` file and click **Open**.
4. Kooka Pictura saves a copy of your current settings, then asks whether to
   quit now or keep working.

The imported settings are applied the next time Kooka Pictura starts. Choose
**Quit Now** to close Kooka Pictura now and start it again to use the imported
settings. Choose **Later** to keep working and apply them on your next start.
The current window is not changed until then.

When you import, Kooka Pictura first writes a copy of your current settings to
the `settings-backups` folder inside its settings folder. If you change your
mind, import that copy to get your old settings back.

## What import does

- Import **replaces** the settings in the file. It does not merge them with
  your current settings.
- A recent-file entry that does not exist on this computer is dropped.
- A saved window position that is not on any screen is ignored, so the window
  cannot open off-screen.
- A file written with a newer backup format is refused. A backup from an older
  version is accepted; settings that did not exist in that version keep their
  defaults.
- A damaged file is refused, and your current settings are left unchanged.

The `pictura.json` file is a backup, not a document. Keep it somewhere safe if
it contains recent-file names you would rather not share.
