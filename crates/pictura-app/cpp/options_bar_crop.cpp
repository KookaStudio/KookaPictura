// The Crop tool's options bar. Split from options_bar.cpp to keep each
// translation unit under the size cap.

#include "options_bar.h"

#include "icons.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QPointF>
#include <QtCore/QSignalBlocker>
#include <QtCore/QVariant>
#include <QtGui/QActionGroup>
#include <QtGui/QIcon>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidgetAction>

namespace pictura {

namespace {

constexpr double kResolutionMode = -100.0;
constexpr double kNewPreset = -1.0;
constexpr double kDeletePreset = -2.0;

// A non-selectable group header inside the ratio combo.
void addComboHeader(QComboBox* combo, const QString& text)
{
    combo->addItem(text);
    const int row = combo->count() - 1;
    if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
        if (QStandardItem* item = model->item(row)) {
            item->setFlags(Qt::NoItemFlags);
        }
    }
}

} // namespace

// CS6's Crop bar: the ratio select with a free `Ratio` default, the W/H fields
// (ratio values, or pixels in the W x H x Resolution mode) with swap, Clear and
// the spirit-level Straighten toggle, grid and settings menus, Delete Cropped
// Pixels and a disabled Content-Aware placeholder, reset, and a cancel/commit
// pair (Esc / Enter do the same). Groups are separated by vertical rules.
QWidget* OptionsBar::buildCropPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    const auto separator = [page]() {
        auto* line = new QFrame(page);
        line->setFrameShape(QFrame::VLine);
        line->setFrameShadow(QFrame::Sunken);
        line->setFixedHeight(18);
        return line;
    };
    layout->addWidget(toolButton(id, page));

    // Each preset carries a QPointF(width, height); the special entries use a
    // negative-x sentinel and the free `Ratio` entry is (0, 0).
    auto* ratio = new QComboBox(page);
    ratio->setObjectName(QStringLiteral("optionsCropRatio"));
    ratio->setToolTip(QStringLiteral("Lock the crop box to an aspect ratio"));
    const auto addPair = [ratio](const QString& label, double w, double h) {
        ratio->addItem(label, QVariant::fromValue(QPointF(w, h)));
    };
    const auto rebuild = [this, ratio, addPair]() {
        const QSignalBlocker block(ratio);
        ratio->clear();
        addPair(QStringLiteral("Ratio"), 0.0, 0.0);
        ratio->insertSeparator(ratio->count());
        addPair(QStringLiteral("W x H x Resolution"), kResolutionMode, 0.0);
        ratio->insertSeparator(ratio->count());
        addComboHeader(ratio, QStringLiteral("Original Ratio"));
        addPair(QStringLiteral("1 : 1 (Square)"), 1, 1);
        addPair(QStringLiteral("4 : 5 (8 : 10)"), 4, 5);
        addPair(QStringLiteral("5 : 7"), 5, 7);
        addPair(QStringLiteral("2 : 3 (4 : 6)"), 2, 3);
        addPair(QStringLiteral("16 : 9"), 16, 9);
        ratio->insertSeparator(ratio->count());
        addComboHeader(ratio, QStringLiteral("Front Image"));
        addPair(QStringLiteral("4 x 5 in 300 ppi"), 4, 5);
        addPair(QStringLiteral("8.5 x 11 in 300 ppi"), 8.5, 11);
        addPair(QStringLiteral("1024 x 768 px 92 ppi"), 1024, 768);
        addPair(QStringLiteral("1280 x 800 px 113 ppi"), 1280, 800);
        addPair(QStringLiteral("1366 x 768 px 135 ppi"), 1366, 768);
        ratio->insertSeparator(ratio->count());
        addPair(QStringLiteral("1 : 2"), 1, 2);
        addPair(QStringLiteral("10 : 7"), 10, 7);
        addPair(QStringLiteral("3 : 4"), 3, 4);
        addPair(QStringLiteral("9 : 19.5 (iPhone 14 Pro Max)"), 9, 19.5);
        for (const auto& [label, pair] : cropPresets_) {
            addPair(label, pair.x(), pair.y());
        }
        ratio->insertSeparator(ratio->count());
        addPair(QStringLiteral("New Crop Preset..."), kNewPreset, 0.0);
        addPair(QStringLiteral("Delete Crop Preset..."), kDeletePreset, 0.0);
    };
    rebuild();
    layout->addWidget(ratio);

    // W/H lose their leading labels and the slider popup; they accept decimals
    // (a ratio like 2.5:1) but format as integers unless a decimal is typed.
    auto* width = new NumericField(QString(),
                                   numericConfig(1.0, 100000.0, 1.0, 0, QStringLiteral(" px"),
                                                 false, QStringLiteral("optionsCropWidth"), true),
                                   page);
    layout->addWidget(width);
    auto* swap = new QToolButton(page);
    swap->setObjectName(QStringLiteral("optionsCropSwap"));
    swap->setIcon(icon(QStringLiteral("crop.swap")));
    swap->setIconSize(QSize(18, 18));
    swap->setToolTip(QStringLiteral("Swap the crop width and height"));
    layout->addWidget(swap);
    auto* height = new NumericField(QString(),
                                    numericConfig(1.0, 100000.0, 1.0, 0, QStringLiteral(" px"),
                                                  false, QStringLiteral("optionsCropHeight"), true),
                                    page);
    layout->addWidget(height);
    cropWidthField_ = width;
    cropHeightField_ = height;
    // Only the W x H x Resolution mode shows the pixel suffix, the resolution
    // field, and its unit; the resolution has no leading `Res:` label.
    auto* resolution = new NumericField(
        QString(),
        numericConfig(1.0, 100000.0, 1.0, 0, QString(), false, QStringLiteral("optionsCropResolution")),
        page);
    layout->addWidget(resolution);
    auto* unit = new QComboBox(page);
    unit->setObjectName(QStringLiteral("optionsCropUnit"));
    unit->addItems({QStringLiteral("px/in"), QStringLiteral("px/cm")});
    // Size the unit combo to its contents so `px/in` is not truncated.
    unit->setSizeAdjustPolicy(QComboBox::AdjustToContents);
    layout->addWidget(unit);
    layout->addWidget(separator());

    auto* clear = new QToolButton(page);
    clear->setObjectName(QStringLiteral("optionsCropClear"));
    clear->setText(QStringLiteral("Clear"));
    clear->setToolTip(QStringLiteral("Clear the aspect ratio"));
    layout->addWidget(clear);

    // The spirit-level: a checkable toggle that arms the draw-a-horizon line tool.
    auto* spirit = new QToolButton(page);
    spirit->setObjectName(QStringLiteral("optionsCropSpirit"));
    spirit->setCheckable(true);
    spirit->setIcon(icon(QStringLiteral("info.protractor")));
    spirit->setIconSize(QSize(18, 18));
    spirit->setToolTip(QStringLiteral("Straighten: drag a line along the horizon"));
    layout->addWidget(spirit);
    layout->addWidget(separator());

    const auto menuIcon = [page](const QString& name, const QString& iconId, const QString& tip) {
        auto* button = new QToolButton(page);
        button->setObjectName(name);
        button->setIcon(icon(iconId));
        button->setIconSize(QSize(18, 18));
        button->setToolTip(tip);
        button->setPopupMode(QToolButton::InstantPopup);
        button->setMenu(new QMenu(button));
        return button;
    };
    // Grid overlay menu: overlay choice, show mode, and cycle hints.
    auto* gridButton = menuIcon(QStringLiteral("optionsCropGrid"), QStringLiteral("crop.grid"),
                                QStringLiteral("Crop guide overlay"));
    {
        QMenu* menu = gridButton->menu();
        auto* group = new QActionGroup(menu);
        group->setExclusive(true);
        const QStringList overlays = {
            QStringLiteral("Rule of Thirds"), QStringLiteral("Grid"),
            QStringLiteral("Diagonal"),       QStringLiteral("Triangle"),
            QStringLiteral("Golden Ratio"),   QStringLiteral("Golden Spiral")};
        for (int i = 0; i < overlays.size(); ++i) {
            QAction* action = menu->addAction(overlays[i]);
            action->setCheckable(true);
            action->setChecked(controller_ ? controller_->cropGridOverlay() == i : i == 0);
            group->addAction(action);
            connect(action, &QAction::triggered, this, [this, i]() {
                if (controller_) {
                    controller_->setCropGridOverlay(i);
                }
            });
        }
        menu->addSeparator();
        auto* showGroup = new QActionGroup(menu);
        showGroup->setExclusive(true);
        for (const QString& item : {QStringLiteral("Auto Show Overlay"),
                                    QStringLiteral("Always Show Overlay"),
                                    QStringLiteral("Never Show Overlay")}) {
            QAction* action = menu->addAction(item);
            action->setCheckable(true);
            action->setChecked(item == QStringLiteral("Always Show Overlay"));
            showGroup->addAction(action);
        }
        menu->addSeparator();
        QAction* cycle = menu->addAction(QStringLiteral("Cycle Overlay"));
        cycle->setShortcut(QKeySequence(QStringLiteral("O")));
        cycle->setShortcutVisibleInContextMenu(true);
        QAction* orient = menu->addAction(QStringLiteral("Cycle Orientation"));
        orient->setShortcut(QKeySequence(QStringLiteral("Shift+O")));
        orient->setShortcutVisibleInContextMenu(true);
        orient->setEnabled(false);
    }
    layout->addWidget(gridButton);
    // Settings (cog) menu: Classic (default) vs Modern crop drag semantics, the
    // shield controls, and display toggles.
    auto* cogButton = menuIcon(QStringLiteral("optionsCropCog"), QStringLiteral("crop.cog"),
                               QStringLiteral("Crop settings"));
    {
        QMenu* menu = cogButton->menu();
        QAction* classic = menu->addAction(QStringLiteral("Use Classic Mode"));
        classic->setCheckable(true);
        classic->setChecked(controller_ ? controller_->cropClassicMode() : true);
        classic->setShortcut(QKeySequence(QStringLiteral("P")));
        classic->setShortcutVisibleInContextMenu(true);
        classic->setToolTip(QStringLiteral(
            "Classic: dragging inside moves the crop box. Modern: it moves the content"));
        connect(classic, &QAction::toggled, this, [this](bool on) {
            if (controller_) {
                controller_->setCropClassicMode(on);
            }
        });
        QAction* showArea = menu->addAction(QStringLiteral("Show Cropped Area"));
        showArea->setCheckable(true);
        showArea->setChecked(true);
        showArea->setShortcut(QKeySequence(QStringLiteral("H")));
        showArea->setShortcutVisibleInContextMenu(true);
        QAction* autoCenter = menu->addAction(QStringLiteral("Auto Center Preview"));
        autoCenter->setCheckable(true);
        autoCenter->setChecked(true);
        menu->addSeparator();
        QAction* shield = menu->addAction(QStringLiteral("Enable Crop Shield"));
        shield->setCheckable(true);
        shield->setChecked(true);
        const auto comboRow = [menu](const QString& label, const QStringList& values,
                                     int current) {
            auto* widget = new QWidget(menu);
            auto* row = new QHBoxLayout(widget);
            row->setContentsMargins(20, 2, 6, 2);
            row->addWidget(new QLabel(label, widget));
            auto* combo = new QComboBox(widget);
            combo->addItems(values);
            combo->setCurrentIndex(current);
            row->addWidget(combo);
            auto* action = new QWidgetAction(menu);
            action->setDefaultWidget(widget);
            menu->addAction(action);
        };
        comboRow(QStringLiteral("Color:"),
                 {QStringLiteral("Match Canvas"), QStringLiteral("Black"), QStringLiteral("White"),
                  QStringLiteral("Custom")},
                 0);
        comboRow(QStringLiteral("Opacity:"),
                 {QStringLiteral("25%"), QStringLiteral("50%"), QStringLiteral("75%"),
                  QStringLiteral("100%")},
                 2);
        QAction* autoOpacity = menu->addAction(QStringLiteral("Auto Adjust Opacity"));
        autoOpacity->setCheckable(true);
        autoOpacity->setChecked(true);
    }
    layout->addWidget(cogButton);
    layout->addWidget(separator());

    auto* deletePixels = new QCheckBox(QStringLiteral("Delete Cropped Pixels"), page);
    deletePixels->setObjectName(QStringLiteral("optionsCropDelete"));
    deletePixels->setChecked(controller_ ? controller_->cropDeletePixels() : true);
    deletePixels->setToolTip(QStringLiteral(
        "Discard the pixels outside the crop rather than keeping them beyond the canvas edge"));
    layout->addWidget(deletePixels);
    auto* contentAware = new QCheckBox(QStringLiteral("Content-Aware"), page);
    contentAware->setObjectName(QStringLiteral("optionsCropContentAware"));
    contentAware->setEnabled(false);
    contentAware->setToolTip(QStringLiteral("Content-Aware crop is not implemented"));
    layout->addWidget(contentAware);
    layout->addWidget(separator());

    auto* reset = new QToolButton(page);
    reset->setObjectName(QStringLiteral("optionsCropReset"));
    reset->setIcon(icon(QStringLiteral("edit.undo")));
    reset->setIconSize(QSize(18, 18));
    reset->setToolTip(QStringLiteral("Reset the crop values"));
    layout->addWidget(reset);
    layout->addWidget(separator());
    auto* cancel = new QToolButton(page);
    cancel->setObjectName(QStringLiteral("optionsCropCancel"));
    cancel->setIcon(icon(QStringLiteral("crop.cancel")));
    cancel->setIconSize(QSize(18, 18));
    cancel->setToolTip(QStringLiteral("Cancel the crop (Esc)"));
    layout->addWidget(cancel);
    auto* commit = new QToolButton(page);
    commit->setObjectName(QStringLiteral("optionsCropCommit"));
    commit->setIcon(icon(QStringLiteral("crop.apply")));
    commit->setIconSize(QSize(18, 18));
    commit->setToolTip(QStringLiteral("Apply the crop (Enter)"));
    layout->addWidget(commit);

    const auto setResolutionFields = [width, height, resolution, unit](bool on) {
        width->setSuffixVisible(on);
        height->setSuffixVisible(on);
        resolution->setVisible(on);
        unit->setVisible(on);
    };
    setResolutionFields(false);

    if (controller_) {
        const auto applyFields = [this, width, height]() {
            if (cropResolutionMode_) {
                controller_->resizeCrop(width->value(), height->value());
            } else {
                controller_->setCropRatio(
                    height->value() > 0.0 ? width->value() / height->value() : 0.0);
            }
        };
        connect(width, &NumericField::valueChanged, this,
                [applyFields](double) { applyFields(); });
        connect(height, &NumericField::valueChanged, this,
                [applyFields](double) { applyFields(); });
        connect(swap, &QToolButton::clicked, this, [this, width, height]() {
            const double w = width->value();
            const double h = height->value();
            if (cropResolutionMode_) {
                controller_->resizeCrop(h, w);
            } else {
                controller_->setCropRatio(w > 0.0 ? h / w : 0.0);
            }
            cropRatioW_ = h;
            cropRatioH_ = w;
            const QSignalBlocker bw(width);
            const QSignalBlocker bh(height);
            width->setValue(h);
            height->setValue(w);
        });
        const auto resetRatio = [this, ratio, width, height]() {
            cropRatioW_ = 1.0;
            cropRatioH_ = 1.0;
            controller_->setCropRatio(0.0);
            ratio->setCurrentIndex(0);
            const QSignalBlocker bw(width);
            const QSignalBlocker bh(height);
            width->setValue(1.0);
            height->setValue(1.0);
        };
        connect(clear, &QToolButton::clicked, this, resetRatio);
        connect(reset, &QToolButton::clicked, this, [this, resetRatio, spirit]() {
            resetRatio();
            controller_->setCropAngle(0.0);
            if (spirit->isChecked()) {
                spirit->setChecked(false);
            }
        });
        connect(spirit, &QToolButton::toggled, this,
                [this](bool on) { controller_->setCropStraightenMode(on); });
        connect(controller_, &ToolController::cropOptionsChanged, this, [this, spirit]() {
            const QSignalBlocker block(spirit);
            spirit->setChecked(controller_->cropStraightenMode());
        });
        // Resample-free resolution: the field defaults to the document's
        // resolution (CS6's 72 ppi fallback). ponytail: metadata only for now;
        // the crop engine isolates pixels and does not resample on commit.
        const auto defaultPpi = [this]() -> double {
            if (PictureView* v = controller_->view(); v && v->has_document()) {
                const double ppi = document_ppi(*v);
                if (ppi > 0.0) {
                    return ppi;
                }
            }
            return 72.0;
        };
        connect(ratio, &QComboBox::currentIndexChanged, this,
                [this, ratio, rebuild, setResolutionFields, width, height, resolution, defaultPpi](
                    int index) {
                    const QPointF pair = ratio->itemData(index).toPointF();
                    if (pair.x() == kResolutionMode) {
                        cropResolutionMode_ = true;
                        setResolutionFields(true);
                        const QSignalBlocker bw(width);
                        const QSignalBlocker bh(height);
                        width->setValue(controller_->cropWidth());
                        height->setValue(controller_->cropHeight());
                        resolution->setValue(defaultPpi());
                        return;
                    }
                    cropResolutionMode_ = false;
                    setResolutionFields(false);
                    if (pair.x() == kNewPreset) {
                        bool ok = false;
                        const QString name = QInputDialog::getText(
                            this, tr("New Crop Preset"), tr("Name:"), QLineEdit::Normal,
                            QString(), &ok);
                        if (ok && !name.isEmpty()) {
                            const double w = width->value() > 0.0 ? width->value() : 1.0;
                            const double h = height->value() > 0.0 ? height->value() : 1.0;
                            cropPresets_.emplace_back(name, QPointF(w, h));
                            rebuild();
                        }
                        return;
                    }
                    if (pair.x() == kDeletePreset) {
                        if (!cropPresets_.empty()) {
                            cropPresets_.pop_back();
                            rebuild();
                        }
                        return;
                    }
                    const double w = pair.x() > 0.0 ? pair.x() : 1.0;
                    const double h = pair.y() > 0.0 ? pair.y() : 1.0;
                    cropRatioW_ = w;
                    cropRatioH_ = h;
                    const QSignalBlocker bw(width);
                    const QSignalBlocker bh(height);
                    width->setValue(w);
                    height->setValue(h);
                    controller_->setCropRatio(
                        pair.x() > 0.0 && pair.y() > 0.0 ? pair.x() / pair.y() : 0.0);
                });
        connect(deletePixels, &QCheckBox::toggled, this,
                [this](bool on) { controller_->setCropDeletePixels(on); });
        connect(cancel, &QToolButton::clicked, this, [this]() { controller_->cancelCrop(); });
        connect(commit, &QToolButton::clicked, this, [this]() { controller_->commitCrop(); });
        // Cancel/Apply/Reset are visible only while a box is active (not in the
        // preview or the boxless init state). Re-synced on every crop-state
        // change, not just the tool switch.
        const auto syncCommitControls = [this, reset, cancel, commit]() {
            const bool on = controller_ && controller_->cropActive();
            reset->setVisible(on);
            cancel->setVisible(on);
            commit->setVisible(on);
        };
        syncCommitControls();
        connect(controller_, &ToolController::toolSessionChanged, this, syncCommitControls);
        connect(controller_, &ToolController::cropOptionsChanged, this, syncCommitControls);
    }
    layout->addStretch(1);
    return page;
}

void OptionsBar::syncCropFields()
{
    if (!controller_ || !cropWidthField_ || !cropHeightField_) {
        return;
    }
    const QSignalBlocker bw(cropWidthField_);
    const QSignalBlocker bh(cropHeightField_);
    if (cropResolutionMode_) {
        cropWidthField_->setValue(controller_->cropWidth());
        cropHeightField_->setValue(controller_->cropHeight());
    } else {
        cropWidthField_->setValue(cropRatioW_);
        cropHeightField_->setValue(cropRatioH_);
    }
}

} // namespace pictura
