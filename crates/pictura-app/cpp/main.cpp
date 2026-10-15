#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPalette>
#include <QtGui/QPixmap>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDockWidget>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <optional>

#include "commands.h"
#include "control_server.h"
#include "dialog_chrome.h"
#include "dialogs.h"
#include "fonts.h"
#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "preferences_dialog.h"
#include "session.h"
#include "theme.h"
#include "toolbox.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "interop.h"
#include "selftest.h"

int main(int argc, char* argv[])
{
    // xvfb-run sets DISPLAY but a live Wayland session's WAYLAND_DISPLAY leaks
    // through, so Qt prefers the Wayland plugin. There a popup opened without a
    // preceding input event cannot grab and the compositor dismisses it, which
    // makes the UI self-tests flaky. Route the self-test back to the X display.
    bool selfTestArg = false;
    bool headlessArg = false;
    for (int i = 1; i < argc; ++i) {
        selfTestArg = selfTestArg || std::strcmp(argv[i], "--self-test") == 0;
        headlessArg = headlessArg || std::strcmp(argv[i], "--headless") == 0
            || std::strcmp(argv[i], "--dump-theme") == 0;
    }
    // A headless or theme-dump run must not need X or Wayland; --headless wins
    // over the self-test xcb routing below because it sets the variable that
    // guard checks.
    if (headlessArg && !qEnvironmentVariableIsSet("QT_QPA_PLATFORM")) {
        qputenv("QT_QPA_PLATFORM", "offscreen");
    }
    if (selfTestArg && qEnvironmentVariableIsSet("DISPLAY")
        && qEnvironmentVariableIsSet("WAYLAND_DISPLAY")
        && !qEnvironmentVariableIsSet("QT_QPA_PLATFORM")) {
        qputenv("QT_QPA_PLATFORM", "xcb");
    }

    // Flatpak/Snap sandboxes reach host files only through the desktop portal.
    // The portal platform theme is authoritative there (it wraps the base theme,
    // and the app forces Fusion), so force it even if the sandbox preset a
    // non-portal theme; a native install keeps the built-in enriched dialog.
    if (pictura::usesPortalFileDialog()) {
        qputenv("QT_QPA_PLATFORMTHEME", "xdgdesktopportal");
    }

    QApplication app(argc, argv);
    QApplication::setWindowIcon(pictura::icon(QStringLiteral("app")));
    // Pin the UI to the bundled Noto Sans so every OS resolves the same family
    // and the weighted chrome (Medium tab labels, etc.) hits a face that exists.
    pictura::applyBundledUiFont();
    pictura::installDialogChrome();

    // Surface Qt's own diagnostics (QRhi logs through qWarning) on stderr so
    // the interop probe can capture them under xvfb/offscreen.
    qInstallMessageHandler([](QtMsgType type, const QMessageLogContext&, const QString& message) {
        std::fprintf(stderr, "qt[%d]: %s\n", static_cast<int>(type), message.toLocal8Bit().constData());
        std::fflush(stderr);
    });

    const QStringList args = app.arguments();
    bool selfTest = false;
    bool interopProbe = false;
    bool headless = false;
    bool control = false;
    bool dumpTheme = false;
    QString psdPath;
    QString controlSocket;
    QString stateHome;
    for (int i = 1; i < args.size(); ++i) {
        if (args.at(i) == QStringLiteral("--self-test")) {
            selfTest = true;
        } else if (args.at(i) == QStringLiteral("--interop-probe")) {
            interopProbe = true;
        } else if (args.at(i) == QStringLiteral("--headless")) {
            headless = true;
        } else if (args.at(i) == QStringLiteral("--dump-theme")) {
            dumpTheme = true;
        } else if (args.at(i) == QStringLiteral("--control")) {
            control = true;
        } else if (args.at(i) == QStringLiteral("--control-socket") && i + 1 < args.size()) {
            controlSocket = args.at(++i);
        } else if (args.at(i) == QStringLiteral("--state-home") && i + 1 < args.size()) {
            stateHome = args.at(++i);
        } else if (!args.at(i).startsWith(QLatin1Char('-'))) {
            psdPath = args.at(i);
        }
    }

    // The theme dump is a machine-readable read of the token contract; it needs
    // no window, session store, or document.
    if (dumpTheme) {
        const auto levelsFor = [](pictura::Theme::ThemeFamily family) {
            QJsonArray levels;
            for (int level = 0; level < pictura::Theme::kLevelCount; ++level) {
                QJsonObject colors;
                QJsonObject metrics;
                for (const pictura::ThemeToken& token : pictura::Theme::tokens(family, level)) {
                    (token.color ? colors : metrics).insert(token.key, token.value);
                }
                QJsonObject entry;
                entry.insert(QStringLiteral("level"), level);
                entry.insert(QStringLiteral("colors"), colors);
                entry.insert(QStringLiteral("metrics"), metrics);
                levels.append(entry);
            }
            return levels;
        };
        QJsonObject root;
        root.insert(QStringLiteral("defaultLevel"), pictura::Theme::kDefaultLevel);
        root.insert(QStringLiteral("levelCount"), pictura::Theme::kLevelCount);
        root.insert(QStringLiteral("levels"), levelsFor(pictura::Theme::kDefaultFamily));
        // Both families, keyed by name, so the dump covers the whole contract.
        QJsonArray families;
        for (const pictura::Theme::ThemeFamily family :
             {pictura::Theme::ThemeFamily::Dark, pictura::Theme::ThemeFamily::Light}) {
            QJsonObject entry;
            entry.insert(QStringLiteral("name"), pictura::Theme::familyName(family));
            entry.insert(QStringLiteral("levels"), levelsFor(family));
            families.append(entry);
        }
        root.insert(QStringLiteral("families"), families);
        const QByteArray json = QJsonDocument(root).toJson(QJsonDocument::Indented);
        std::fwrite(json.constData(), 1, static_cast<std::size_t>(json.size()), stdout);
        std::fflush(stdout);
        return 0;
    }

    // A bare --headless run has nothing to show and no way to quit; run the
    // bridge self-test instead of blocking in the event loop. Control mode keeps
    // its own event loop, so it must not be turned into a self-test.
    if (headless && !selfTest && !control && psdPath.isEmpty()) {
        selfTest = true;
    }

    // Isolate the session store before the frame restores state (session.cpp
    // only reads XDG_STATE_HOME). The self-test's temp dir below wins over a
    // user-supplied --state-home.
    if (!stateHome.isEmpty()) {
        qputenv("XDG_STATE_HOME", stateHome.toUtf8());
    }

    // Self-test must not read the user's saved layout: isolate the session store
    // (session.cpp only reads XDG_STATE_HOME) before the frame restores state.
    std::optional<QTemporaryDir> selfTestStateDir;
    if (selfTest) {
        selfTestStateDir.emplace();
        if (!selfTestStateDir->isValid()) {
            // Never fall back to the real store: a self-test run must not write
            // the user's session.
            std::fprintf(stderr,
                         "pictura self-test: FAIL core state_home_temp_dir 199 "
                         "cannot create the self-test state dir\n");
            std::fflush(stderr);
            return 199;
        }
        qputenv("XDG_STATE_HOME", selfTestStateDir->path().toUtf8());
    }

    if (interopProbe) {
        pictura::PictureView probe;
        const bool prepared = probe.gpu_interop_prepare();
        if (!prepared) {
            std::fprintf(stderr, "pictura interop-probe: no Vulkan device\n");
            return 0;
        }
        const std::int32_t result = pictura_try_qrhi_import(probe.gpu_vk_instance(),
                                                            probe.gpu_vk_physical_device(),
                                                            probe.gpu_vk_device(),
                                                            probe.gpu_vk_queue_family(),
                                                            probe.gpu_vk_image(),
                                                            probe.gpu_image_width(),
                                                            probe.gpu_image_height());
        std::fprintf(stderr, "pictura interop-probe: qrhi_import=%d\n", result);
        std::fflush(stderr);
        return 0;
    }

    pictura::PicturaMainWindow frame;
    // Crash recovery runs only for an interactive launch: the self-test, a
    // headless run, and control mode must neither write snapshots nor raise the
    // recovery prompt.
    const bool recovery = !selfTest && !headless && !control;
    if (recovery) {
        frame.enableRecovery();
    }
    // `File > Exit` / Ctrl+Q call `qApp->quit()`, which leaves the event loop
    // without running `closeEvent`; save on the application's quit signal so
    // every quit path persists the session. The `closeEvent` save is unchanged
    // (and idempotent).
    QObject::connect(&app, &QApplication::aboutToQuit, &frame,
                     &pictura::PicturaMainWindow::saveSession);

    bool codecLoaded = false;
    if (!psdPath.isEmpty()) {
        codecLoaded = frame.openPath(psdPath);
    }
    // A normal launch opens no document; only the self-test keeps the scratch
    // white document its null_image/fresh_white/gpu_blank checks need. The GPU
    // smoke probe reports 0 = no GPU, 1 = rendered non-blank, 2 = blank. The
    // probe never replaces the document image (E1), so the scratch document
    // presents pure white rather than the probe's gradient.
    int gpu = 0;
    if (pictura::launchCreatesScratchDocument(selfTest, codecLoaded)) {
        frame.newDocument(QStringLiteral("Untitled"), 512, 512, QStringLiteral("rgb"), 8,
                          QStringLiteral("white"));
        if (pictura::PictureView* scratch = frame.activeView()) {
            gpu = scratch->render_gpu();
            frame.refresh();
        }
    }
    pictura::PictureView* view = frame.activeView();
    const QImage image = view ? view->image() : QImage();

    // The frame applies the default size (or the stored geometry) in its
    // constructor; resizing here would clobber the restored window geometry.
    frame.show();
    if (recovery) {
        QTimer::singleShot(0, &frame, &pictura::PicturaMainWindow::offerRecovery);
    }

    if (selfTest) {
        const int rc = runSelfTest(app, headless, psdPath, frame, view, image,
                                   codecLoaded, gpu);
        if (rc != 0) {
            return rc;
        }
    }

    if (control) {
        // No modal may be reachable from a control request.
        pictura::setUnsavedPromptInteractive(false);
        pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Discard);
        pictura::ControlServer controlServer(&frame, controlSocket);
        if (!controlServer.listen()) {
            std::fprintf(stderr, "pictura control: FAIL: cannot listen on %s\n",
                         qPrintable(controlServer.socketPath()));
            std::fflush(stderr);
            return 153;
        }
        std::fprintf(stderr, "pictura control: %s\n", qPrintable(controlServer.socketPath()));
        std::fflush(stderr);
        QObject::connect(&app, &QApplication::aboutToQuit, &controlServer,
                         &pictura::ControlServer::shutdown);
        return app.exec();
    }

    return app.exec();
}
