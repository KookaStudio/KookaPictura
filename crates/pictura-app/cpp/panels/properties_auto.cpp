#include "properties_panel.h"

#include "auto_correction_options.h"
#include "dialogs.h"
#include "preferences.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPointer>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {
namespace {

class PropertiesAutoButton : public QPushButton {
public:
    PropertiesAutoButton(QWidget* parent, std::function<void()> openOptions)
        : QPushButton(QCoreApplication::translate("PicturaUi", "Auto"), parent)
        , openOptions_(std::move(openOptions))
    {
    }

protected:
    void mouseReleaseEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::LeftButton && isDown()
            && event->modifiers().testFlag(Qt::AltModifier)
            && hitButton(event->position().toPoint())) {
            setDown(false);
            event->accept();
            openOptions_();
            return;
        }
        QPushButton::mouseReleaseEvent(event);
    }

private:
    std::function<void()> openOptions_;
};

QByteArray bytes(const rust::Vec<std::uint8_t>& block)
{
    return QByteArray(reinterpret_cast<const char*>(block.data()), qsizetype(block.size()));
}

rust::Slice<const std::uint8_t> slice(const QByteArray& block)
{
    return {reinterpret_cast<const std::uint8_t*>(block.constData()), std::size_t(block.size())};
}

} // namespace

QPushButton* PropertiesPanel::makeAutoButton(QWidget* parent)
{
    auto* button = new PropertiesAutoButton(parent, [this] { applyAuto(true); });
    button->setObjectName(QStringLiteral("propertiesAuto"));
    button->setToolTip(QCoreApplication::translate("PicturaUi", "Alt-click for Auto Options"));
    button->hide();
    connect(button, &QPushButton::clicked, this, [this] { applyAuto(false); });
    return button;
}

bool PropertiesPanel::canAutoCorrect() const
{
    return view_ && !path_.isEmpty() && !autoDialogOpen_
        && (kind_ == QLatin1String("levels") || kind_ == QLatin1String("curves"));
}

void PropertiesPanel::applyAuto(bool showOptions)
{
    if (!canAutoCorrect()) {
        return;
    }
    commit();

    const QPointer<PictureView> view(view_);
    const QString path = path_;
    const QString title = title_;
    const QByteArray opening = bytes(adjustment_block(*view, path));
    if (opening.isEmpty()) {
        return;
    }
    const auto source = adjustment_auto_source(*view, path);
    const auto saved = autoCorrectionSettings(loadPreferences());
    QByteArray shown = opening;
    bool previewTouched = false;
    const auto preview = [&](const AutoCorrectionSettings& settings) {
        if (!view || view_ != view.data() || path_ != path) {
            return;
        }
        auto block = bytes(adjustment_auto_block(*source, slice(opening), settings));
        if (block.isEmpty()) {
            block = opening;
        }
        if (shown != block && adjustment_set_block(*view, path, slice(block))) {
            shown = std::move(block);
            previewTouched = true;
        }
    };

    bool accepted = true;
    if (showOptions) {
        autoDialogOpen_ = true;
        AutoCorrectionOptionsDialog dialog(saved, preview, window());
        preview(saved);
        accepted = runDialog(dialog, window()) == QDialog::Accepted;
        autoDialogOpen_ = false;
        if (accepted) {
            const auto settings = dialog.settings();
            preview(settings);
            if (dialog.saveAsDefaults() && !saveAutoCorrectionSettings(settings)) {
                QMessageBox::warning(window(), dialog.windowTitle(),
                    QCoreApplication::translate("PicturaUi", "Auto correction defaults could not be saved."));
            }
        }
    } else {
        preview(saved);
    }

    if (view && previewTouched) {
        if (accepted && view_ == view.data() && path_ == path && shown != opening) {
            adjustment_commit(*view, title);
        } else {
            adjustment_cancel(*view);
        }
    }
    refresh();
}

} // namespace pictura
