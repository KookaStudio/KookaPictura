#include "api/layers_bridge.h"
#include "color_picker_dialog.h"
#include "frame_includes.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "panels/color_swatch_widget.h"
#include <QtCore/QScopedValueRollback>
#include <QtCore/QSignalBlocker>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTextEdit>
#include <QtWidgets/QTreeView>

namespace pictura {
namespace {
    const QString maskCommand = QString::fromLatin1(command_ids::ViewQuickMask);
    class QuickMaskKeys final : public QObject {
    public:
        explicit QuickMaskKeys(PicturaMainWindow* frame)
            : QObject(frame)
            , frame_(frame)
        {
            qApp->installEventFilter(this);
        }

    protected:
        bool eventFilter(QObject* target, QEvent* event) override
        {
            if (event->type() != QEvent::KeyPress && event->type() != QEvent::ShortcutOverride)
                return false;
            auto* widget = qobject_cast<QWidget*>(target);
            if (!widget || (widget != frame_ && !frame_->isAncestorOf(widget)) || QApplication::activeModalWidget())
                return false;
            for (QWidget* focus = QApplication::focusWidget(); focus;
                focus = focus->parentWidget()) {
                if (qobject_cast<QLineEdit*>(focus) || qobject_cast<QTextEdit*>(focus) || qobject_cast<QPlainTextEdit*>(focus))
                    return false;
            }
            auto* key = static_cast<QKeyEvent*>(event);
            if (key->key() != Qt::Key_AsciiTilde)
                return false;
            if (event->type() == QEvent::ShortcutOverride) {
                event->accept();
                return true;
            }
            if (key->isAutoRepeat())
                return true;
            if (auto* view = frame_->activeView();
                view && quick_mask_active(*view)) {
                quick_mask_set_grayscale(*view, !quick_mask_grayscale(*view));
            }
            return true;
        }

    private:
        PicturaMainWindow* frame_;
    };
} // namespace

void PicturaMainWindow::wireQuickMask()
{
    const auto state = loadSession();
    quickMaskUi_.selected = state.quickMaskSelected;
    quickMaskUi_.color = QColor(state.quickMaskColor);
    quickMaskUi_.opacity = state.quickMaskOpacity;
    if (QAction* action = commandRegistry()->action(maskCommand)) {
        action->setAutoRepeat(false);
        for (QObject* object : action->associatedObjects()) {
            if (auto* menu = qobject_cast<QMenu*>(object))
                menu->removeAction(action);
        }
        addAction(action);
    }
    commandRegistry()->setHandler(maskCommand, [this] {
        if (auto* view = activeView())
            quick_mask_set(*view, !quick_mask_active(*view));
    });
    commandRegistry()->setEnabledProvider(maskCommand, [this] {
        return activeView() && activeView()->has_document();
    });
    commandRegistry()->setCheckedProvider(maskCommand, [this] {
        return activeView() && quick_mask_active(*activeView());
    });
    new QuickMaskKeys(this);
    connect(toolbox_, &Toolbox::paintMaskToggled, this, [this](bool) {
        if (activeView() && activeView()->has_document())
            commandRegistry()->dispatch(maskCommand);
    });
    connect(toolbox_, &Toolbox::quickMaskOptionsRequested, this,
        &PicturaMainWindow::showQuickMaskOptions);
    connect(toolbox_, &Toolbox::quickMaskPolarityRequested, this, [this] {
        if (auto* view = activeView())
            quick_mask_set_options(*view, !quick_mask_selected_areas(*view),
                quick_mask_color(*view),
                quick_mask_opacity(*view));
    });
    syncQuickMask();
}

void PicturaMainWindow::syncQuickMask()
{
    if (quickMaskUi_.syncing || !toolbox_ || !panels_->colorState())
        return;
    QScopedValueRollback<bool> syncing(quickMaskUi_.syncing, true);
    auto* view = activeView();
    const bool active = view && quick_mask_active(*view);
    if (view && quickMaskUi_.settingsView != view) {
        quickMaskUi_.settingsView = view;
        quick_mask_set_options(*view, quickMaskUi_.selected,
            quickMaskUi_.color.rgba(), quickMaskUi_.opacity);
    } else if (view) {
        quickMaskUi_.selected = quick_mask_selected_areas(*view);
        quickMaskUi_.color = QColor::fromRgba(quick_mask_color(*view));
        quickMaskUi_.opacity = quick_mask_opacity(*view);
    }
    if (quickMaskUi_.colorsActive && (!active || quickMaskUi_.colorView != view)) {
        if (quickMaskUi_.colorView) {
            quickMaskUi_.colorView->setProperty("quickMaskForeground",
                panels_->colorState()->foreground());
            quickMaskUi_.colorView->setProperty("quickMaskBackground",
                panels_->colorState()->background());
            if (!quick_mask_active(*quickMaskUi_.colorView)) {
                quickMaskUi_.colorView->setProperty("quickMaskForeground", QVariant());
                quickMaskUi_.colorView->setProperty("quickMaskBackground", QVariant());
            }
        }
        panels_->colorState()->setMaskActive(quickMaskUi_.returnMask);
        panels_->colorState()->setForeground(quickMaskUi_.returnForeground);
        panels_->colorState()->setBackground(quickMaskUi_.returnBackground);
        quickMaskUi_.colorsActive = false;
    }
    if (!quickMaskUi_.colorsActive) {
        const bool mask = view && !mask_edit_target_get(*view).isEmpty();
        if (panels_->colorState()->maskActive() != mask) {
            const QColor fg = panels_->colorState()->foreground(),
                         bg = panels_->colorState()->background();
            panels_->colorState()->setForeground(otherForeground_);
            panels_->colorState()->setBackground(otherBackground_);
            otherForeground_ = fg;
            otherBackground_ = bg;
            panels_->colorState()->setMaskActive(mask);
        }
        if (active) {
            if (tools_)
                tools_->onSessionCancelled();
            quickMaskUi_.returnMask = panels_->colorState()->maskActive();
            quickMaskUi_.returnForeground = panels_->colorState()->foreground();
            quickMaskUi_.returnBackground = panels_->colorState()->background();
            quickMaskUi_.colorView = view;
            panels_->colorState()->setMaskActive(true);
            panels_->colorState()->setForeground(
                view->property("quickMaskForeground").value<QColor>().isValid()
                    ? view->property("quickMaskForeground").value<QColor>()
                    : QColor(Qt::black));
            panels_->colorState()->setBackground(
                view->property("quickMaskBackground").value<QColor>().isValid()
                    ? view->property("quickMaskBackground").value<QColor>()
                    : QColor(Qt::white));
            quickMaskUi_.colorsActive = true;
        }
    }
    toolbox_->setQuickMaskState(view && view->has_document(), active,
        view && quick_mask_selected_areas(*view));
    if (tools_) {
        tools_->setMaskActive(active || (view && !mask_edit_target_get(*view).isEmpty()));
        tools_->refreshCursor();
    }
    if (panels_->layersPanel()) {
        if (auto* tree = panels_->layersPanel()->findChild<QTreeView*>()) {
            tree->setProperty("quickMaskActivePath",
                active ? layer_selection_anchor(*view) : QString());
            tree->viewport()->update();
        }
    }
}

void PicturaMainWindow::showQuickMaskOptions()
{
    auto* view = activeView();
    if (!view)
        return;
    QDialog dialog(this);
    dialog.setObjectName(QStringLiteral("quickMaskOptionsDialog"));
    dialog.setWindowTitle(tr("Quick Mask Options"));
    auto* layout = new QFormLayout(&dialog);
    auto* masked = new QRadioButton(tr("Masked Areas"), &dialog);
    auto* selected = new QRadioButton(tr("Selected Areas"), &dialog);
    masked->setObjectName(QStringLiteral("quickMaskMaskedAreas"));
    selected->setObjectName(QStringLiteral("quickMaskSelectedAreas"));
    selected->setChecked(quick_mask_selected_areas(*view));
    masked->setChecked(!selected->isChecked());
    layout->addRow(tr("Color Indicates:"), masked);
    layout->addRow(QString(), selected);
    auto* color = new ColorSwatchWidget(&dialog);
    color->setObjectName(QStringLiteral("quickMaskColor"));
    color->setColor(QColor::fromRgba(quick_mask_color(*view)));
    connect(color, &QPushButton::clicked, &dialog, [color, &dialog] {
        const QColor picked = ColorPickerDialog::getColor(color->color(), &dialog, tr("Mask Color"));
        if (picked.isValid())
            color->setColor(picked);
    });
    layout->addRow(tr("Color:"), color);
    auto* opacity = new QSpinBox(&dialog);
    opacity->setObjectName(QStringLiteral("quickMaskOpacity"));
    opacity->setRange(0, 100);
    opacity->setSuffix(QStringLiteral("%"));
    opacity->setValue(quick_mask_opacity(*view));
    layout->addRow(tr("Opacity:"), opacity);
    auto* buttons = new QDialogButtonBox(
        QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addRow(buttons);
    if (dialog.exec() == QDialog::Accepted) {
        quick_mask_set_options(*view, selected->isChecked(), color->color().rgba(),
            opacity->value());
        saveSession();
    }
}
} // namespace pictura
