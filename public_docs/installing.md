# Installing

Use the **Download** button on the home page. It lists the installers for the
latest release. You can also find them on the
[releases page](https://github.com/KookaStudio/KookaPictura/releases/latest).

## Linux

Pick the format that suits your system:

| Format | Good for |
|---|---|
| AppImage | Any distribution. Make the file executable and run it. |
| deb | Debian, Ubuntu and derivatives. |
| rpm | Fedora, RHEL and derivatives. |
| snap | Systems with snap installed. |
| flatpak | Systems with Flatpak installed. |

## macOS

Download the `.dmg`, open it and drag the app to Applications. The app is not
signed or notarized, so macOS blocks it the first time:

1. Open **System Settings**.
2. Click **Privacy & Security**, scroll down and click **Open Anyway**.
3. Click **Open**.

After this, you can open the app by double-clicking it.

## Windows

Download and run the `-setup.exe` installer. If Windows only allows apps from
the Microsoft Store, change that first:

1. Open **Settings** from the Start menu.
2. Select **Apps**, then **Apps & features**.
3. Under **Installing apps**, select **Allow apps from anywhere**.

A portable `.zip` is also published on the releases page.

## Building from source

Developers can build the app with Rust, Qt 6, CMake and Ninja. The steps are in
[DEVELOPING.md](https://github.com/KookaStudio/KookaPictura/blob/master/DEVELOPING.md).
