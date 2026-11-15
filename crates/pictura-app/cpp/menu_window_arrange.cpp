// Window > Arrange command registration. The window operations live in
// DocumentRegistry; this file wires the command ids to them.

#include "menu_registrar.h"

#include "document_registry.h"

#include "commands.h"

namespace pictura {

namespace {

const QString kArrangeCascade = QStringLiteral("window.arrange.cascade");
const QString kArrangeTile = QStringLiteral("window.arrange.tile");
const QString kArrangeConsolidate = QStringLiteral("window.arrange.consolidate.all.to.tabs");
const QString kArrangeFloat = QStringLiteral("window.arrange.float.in.window");
const QString kArrangeFloatAll = QStringLiteral("window.arrange.float.all.in.windows");
const QString kArrangeMatchZoom = QStringLiteral("window.arrange.match.zoom");
const QString kArrangeMatchLocation = QStringLiteral("window.arrange.match.location");
const QString kArrangeMatchAll = QStringLiteral("window.arrange.match.all");
const QString kArrangeNewWindow = QStringLiteral("window.arrange.new.window.for.file");

} // namespace

void MenuRegistrar::wireWindowArrange()
{
    const auto hasDocuments = [this]() { return documents_ && documents_->count() > 0; };
    commands_->setHandler(kArrangeFloat,
                          [this]() { documents_->floatWindow(documents_->activeWindow()); });
    commands_->setEnabledProvider(kArrangeFloat, [this]() {
        DocWindow* active = documents_ ? documents_->activeWindow() : nullptr;
        return active && documents_->windows().contains(active)
            && active->kind != DocWindow::Host::Float && !active->doc->loading;
    });
    commands_->setHandler(kArrangeFloatAll, [this]() { documents_->floatAllWindows(); });
    commands_->setEnabledProvider(kArrangeFloatAll, hasDocuments);
    commands_->setHandler(kArrangeConsolidate,
                          [this]() { consolidateAllToTabs(nullptr); });
    commands_->setEnabledProvider(kArrangeConsolidate, hasDocuments);
    commands_->setHandler(kArrangeCascade, [this]() { documents_->cascadeWindows(); });
    commands_->setEnabledProvider(kArrangeCascade, hasDocuments);
    commands_->setHandler(kArrangeTile, [this]() { documents_->tileWindows(); });
    commands_->setEnabledProvider(
        kArrangeTile, [this]() { return documents_ && documents_->windows().size() > 1; });

    const auto canMatch = [this]() {
        DocWindow* active = documents_ ? documents_->activeWindow() : nullptr;
        return documents_ && documents_->windows().size() > 1 && active && active->canvas;
    };
    commands_->setHandler(kArrangeMatchZoom,
                          [this]() { documents_->matchWindows(true, false); });
    commands_->setEnabledProvider(kArrangeMatchZoom, canMatch);
    commands_->setHandler(kArrangeMatchLocation,
                          [this]() { documents_->matchWindows(false, true); });
    commands_->setEnabledProvider(kArrangeMatchLocation, canMatch);
    commands_->setHandler(kArrangeMatchAll,
                          [this]() { documents_->matchWindows(true, true); });
    commands_->setEnabledProvider(kArrangeMatchAll, canMatch);

    commands_->setHandler(kArrangeNewWindow, [this]() {
        if (DocWindow* active = documents_ ? documents_->activeWindow() : nullptr) {
            documents_->addDocumentWindow(active->doc);
        }
    });
    commands_->setEnabledProvider(kArrangeNewWindow, [this]() {
        DocWindow* active = documents_ ? documents_->activeWindow() : nullptr;
        return active && !active->doc->loading;
    });
    commands_->setLabelProvider(kArrangeNewWindow, [this]() {
        const QString label = tr("New Window For");
        DocWindow* active = documents_ ? documents_->activeWindow() : nullptr;
        if (!active) {
            return label;
        }
        QString name = documentDisplayName(*active->doc);
        return label + QLatin1Char(' ') + name.replace(QLatin1Char('&'), QStringLiteral("&&"));
    });
}

} // namespace pictura
