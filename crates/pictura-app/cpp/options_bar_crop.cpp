// The Crop tool's options bar. Split from options_bar.cpp to keep each
// translation unit under the size cap.

#include "options_bar.h"

#include "color_picker_dialog.h"
#include "icons.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QPointF>
#include <QtCore/QSignalBlocker>
#include <QtCore/QVariant>
#include <QtGui/QIcon>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidgetAction>

#include <cmath>
#include <functional>

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

// The Settings menu is one panel of real checkboxes, like CS6's: every row
// starts at the same inset, and a shortcut letter sits in the right-hand column.
QCheckBox* addCheckRow(QWidget* panel, QVBoxLayout* column, const char* name, const QString& text,
                       const QString& keyHint, int indent)
{
    auto* box = new QCheckBox(text, panel);
    box->setObjectName(QLatin1String(name));
    box->setStyleSheet(QStringLiteral("QCheckBox { padding: 4px 14px 4px %1px; }").arg(indent));
    if (!keyHint.isEmpty()) {
        auto* inside = new QHBoxLayout(box);
        inside->setContentsMargins(0, 0, 14, 0);
        inside->addStretch(1);
        auto* hint = new QLabel(keyHint, box);
        hint->setAttribute(Qt::WA_TransparentForMouseEvents);
        inside->addWidget(hint);
    }
    column->addWidget(box);
    return box;
}

void addMenuRule(QWidget* panel, QVBoxLayout* column)
{
    auto* rule = new QFrame(panel);
    rule->setObjectName(QStringLiteral("optionsToolSeparator"));
    rule->setFrameShape(QFrame::HLine);
    rule->setFixedHeight(1);
    column->addSpacing(4);
    column->addWidget(rule);
    column->addSpacing(4);
}

QVBoxLayout* addMenuPanel(QMenu* menu, const QString& name)
{
    auto* panel = new QWidget(menu);
    panel->setObjectName(name);
    auto* column = new QVBoxLayout(panel);
    column->setContentsMargins(0, 4, 0, 6);
    column->setSpacing(0);
    auto* panelAction = new QWidgetAction(menu);
    panelAction->setDefaultWidget(panel);
    menu->addAction(panelAction);
    return column;
}

constexpr int kRowInset = 12;

// The grid menu: plain items picked by clicking, like CS6's: the overlays (each
// with its icon), the show modes, and the cycle commands. The button shows the
// selected overlay's icon. The O / Shift+O keys themselves are taken by
// ToolController::handleCropKey, so the menu shows them as text only: a real
// action shortcut would collide with the tool-letter shortcut.
void buildCropGridMenu(QMenu* menu, QToolButton* button, ToolController* controller,
                       CropOptions* crop)
{
    menu->setObjectName(QStringLiteral("optionsCropGridMenu"));
    const QStringList& names = CropOptions::overlayNames();
    for (int i = 0; i < names.size(); ++i) {
        QAction* action = menu->addAction(icon(CropOptions::overlayIcon(i)), names[i]);
        QObject::connect(action, &QAction::triggered, menu, [crop, button, i]() {
            button->setIcon(icon(CropOptions::overlayIcon(i)));
            if (crop) {
                crop->setGridOverlay(i);
            }
        });
    }
    menu->addSeparator();
    const QStringList shows = {QStringLiteral("Auto Show Overlay"),
                               QStringLiteral("Always Show Overlay"),
                               QStringLiteral("Never Show Overlay")};
    for (int mode = 0; mode < shows.size(); ++mode) {
        QAction* action = menu->addAction(shows[mode]);
        QObject::connect(action, &QAction::triggered, menu, [crop, mode]() {
            if (crop) {
                CropSettings settings = crop->settings();
                settings.overlayShow = mode;
                crop->setSettings(settings);
            }
        });
    }
    menu->addSeparator();
    QAction* cycle = menu->addAction(QStringLiteral("Cycle Overlay\tO"));
    QObject::connect(cycle, &QAction::triggered, menu, [crop]() {
        if (crop) {
            crop->cycleOverlay();
        }
    });
    QAction* orient = menu->addAction(QStringLiteral("Cycle Orientation\tShift+O"));
    QObject::connect(orient, &QAction::triggered, menu, [crop]() {
        if (crop) {
            crop->cycleOverlayOrientation();
        }
    });

    const auto sync = [button, crop, orient]() {
        const int overlay = crop ? crop->gridOverlay() : 0;
        button->setIcon(icon(CropOptions::overlayIcon(overlay)));
        orient->setEnabled(crop && CropOptions::overlayHasOrientation(overlay));
    };
    sync();
    if (controller) {
        QObject::connect(controller, &ToolController::cropOptionsChanged, menu, sync);
    }
}

void buildCropSettingsMenu(QMenu* menu, ToolController* controller, CropOptions* crop)
{
    menu->setObjectName(QStringLiteral("optionsCropSettingsMenu"));
    QVBoxLayout* column = addMenuPanel(menu, QStringLiteral("optionsCropSettingsPanel"));
    QWidget* panel = column->parentWidget();
    const auto checkRow = [panel, column](const char* name, const QString& text,
                                          const QString& keyHint, int indent) {
        return addCheckRow(panel, column, name, text, keyHint, indent);
    };
    QCheckBox* classic = checkRow("cropClassicMode", QStringLiteral("Use Classic Mode"),
                                  QStringLiteral("P"), kRowInset);
    classic->setToolTip(QStringLiteral(
        "Classic: dragging inside moves the crop box. Modern: it moves the content"));
    QCheckBox* showArea =
        checkRow("cropShowArea", QStringLiteral("Show Cropped Area"), QStringLiteral("H"), kRowInset);
    QCheckBox* autoCenter =
        checkRow("cropAutoCenter", QStringLiteral("Auto Center Preview"), QString(), kRowInset);
    addMenuRule(panel, column);
    QCheckBox* shield =
        checkRow("cropShieldEnabled", QStringLiteral("Enable Crop Shield"), QString(), kRowInset);

    auto* shieldBody = new QWidget(panel);
    shieldBody->setObjectName(QStringLiteral("optionsCropShieldPanel"));
    auto* grid = new QGridLayout(shieldBody);
    grid->setContentsMargins(14, 2, 14, 2);
    grid->setHorizontalSpacing(6);
    grid->setVerticalSpacing(6);
    auto* color = new QComboBox(shieldBody);
    color->setObjectName(QStringLiteral("cropShieldColor"));
    color->addItems({QStringLiteral("Match Canvas"), QStringLiteral("Custom")});
    auto* swatch = new QToolButton(shieldBody);
    swatch->setObjectName(QStringLiteral("cropShieldSwatch"));
    swatch->setFixedSize(20, 20);
    NumericFieldConfig opacityConfig;
    opacityConfig.minimum = 0.0;
    opacityConfig.maximum = 100.0;
    opacityConfig.suffix = QStringLiteral("%");
    opacityConfig.popup = true;
    opacityConfig.objectName = QStringLiteral("cropShieldOpacity");
    opacityConfig.namePrefix = QStringLiteral("cropShieldOpacity");
    auto* opacity = new NumericField(QStringLiteral("Opacity:"), opacityConfig, shieldBody);
    opacity->setLabelAlignment(Qt::AlignRight | Qt::AlignVCenter);
    auto* colorLabel = new QLabel(QStringLiteral("Color:"), shieldBody);
    colorLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    grid->addWidget(colorLabel, 0, 0);
    grid->addWidget(color, 0, 1);
    grid->addWidget(swatch, 0, 2);
    grid->addWidget(opacity, 1, 0, 1, 3);
    column->addWidget(shieldBody);
    QCheckBox* autoAdjust =
        checkRow("cropShieldAutoAdjust", QStringLiteral("Auto Adjust Opacity"), QString(), 34);

    const auto edit = [crop](const std::function<void(CropSettings&)>& change) {
        CropSettings settings = crop->settings();
        change(settings);
        crop->setSettings(settings);
    };
    // The project's own picker, as every other colour control uses.
    const auto pickColor = [crop, edit, menu]() {
        QWidget* owner = menu->parentWidget();
        menu->close();
        const QColor picked = ColorPickerDialog::getColor(
            crop->settings().shieldColor, owner, QStringLiteral("Crop Shield Color"));
        if (picked.isValid()) {
            edit([&picked](CropSettings& s) {
                s.shieldCustomColor = true;
                s.shieldColor = picked;
            });
        }
    };

    const QList<QObject*> synced = {classic, showArea, autoCenter, shield, color, opacity, autoAdjust};
    const auto sync = [=]() {
        const CropSettings& s = crop->settings();
        const bool modern = !crop->classicMode();
        for (QObject* o : synced) {
            o->blockSignals(true);
        }
        classic->setChecked(crop->classicMode());
        showArea->setChecked(modern && s.showCroppedArea);
        showArea->setEnabled(modern);
        autoCenter->setChecked(modern && s.autoCenterPreview);
        autoCenter->setEnabled(modern);
        shield->setChecked(s.shieldEnabled);
        color->setCurrentIndex(s.shieldCustomColor ? 1 : 0);
        opacity->setValue(s.shieldOpacity);
        autoAdjust->setChecked(s.autoAdjustOpacity);
        shieldBody->setEnabled(s.shieldEnabled);
        autoAdjust->setEnabled(s.shieldEnabled);
        swatch->setEnabled(s.shieldEnabled && s.shieldCustomColor);
        swatch->setStyleSheet(
            QStringLiteral("background: %1; border: 1px solid #888;")
                .arg(s.shieldCustomColor ? s.shieldColor.name() : QStringLiteral("transparent")));
        for (QObject* o : synced) {
            o->blockSignals(false);
        }
    };
    sync();
    QObject::connect(controller, &ToolController::cropOptionsChanged, menu, sync);

    QObject::connect(classic, &QCheckBox::toggled, menu, [crop](bool on) { crop->setClassicMode(on); });
    QObject::connect(showArea, &QCheckBox::toggled, menu, [edit](bool on) {
        edit([on](CropSettings& s) { s.showCroppedArea = on; });
    });
    QObject::connect(autoCenter, &QCheckBox::toggled, menu, [edit](bool on) {
        edit([on](CropSettings& s) { s.autoCenterPreview = on; });
    });
    QObject::connect(shield, &QCheckBox::toggled, menu, [edit](bool on) {
        edit([on](CropSettings& s) { s.shieldEnabled = on; });
    });
    QObject::connect(color, &QComboBox::activated, menu, [edit, pickColor, sync](int index) {
        if (index == 1) {
            pickColor();
        } else {
            edit([](CropSettings& s) { s.shieldCustomColor = false; });
        }
        sync();
    });
    QObject::connect(swatch, &QToolButton::clicked, menu, pickColor);
    QObject::connect(opacity, &NumericField::valueChanged, menu, [edit](double value) {
        edit([value](CropSettings& s) { s.shieldOpacity = int(std::lround(value)); });
    });
    QObject::connect(autoAdjust, &QCheckBox::toggled, menu, [edit](bool on) {
        edit([on](CropSettings& s) { s.autoAdjustOpacity = on; });
    });
}

} // namespace

// CS6's Crop bar: the ratio select with a free `Ratio` default, the W/H fields
// (ratio values, or pixels in the W x H x Resolution mode) with swap, Clear and
// the spirit-level Straighten toggle, grid and settings menus, Delete Cropped
// Pixels, reset, and a cancel/commit pair (Esc / Enter do the same). Groups are
// separated by vertical rules.
QWidget* OptionsBar::buildCropPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    const auto separator = [page]() {
        auto* line = new QFrame(page);
        line->setObjectName(QStringLiteral("optionsToolSeparator"));
        line->setFrameShape(QFrame::VLine);
        line->setFrameShadow(QFrame::Plain);
        line->setFixedWidth(1);
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
    // The free Ratio default is unconstrained, so the W/H boxes start empty.
    width->setBlank(true);
    height->setBlank(true);
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

    // The spirit-level: a checkable toggle that arms the draw-a-horizon line
    // tool, labelled to its right so the icon and the label both toggle it.
    auto* spirit = new QToolButton(page);
    spirit->setObjectName(QStringLiteral("optionsCropSpirit"));
    spirit->setCheckable(true);
    spirit->setIcon(icon(QStringLiteral("crop.straighten")));
    spirit->setIconSize(QSize(18, 18));
    spirit->setText(QStringLiteral("Straighten"));
    spirit->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    spirit->setToolTip(
        QStringLiteral("Straighten: drag a line along the horizon (hold Ctrl to invoke)"));
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
    auto* gridButton = menuIcon(QStringLiteral("optionsCropGrid"),
                                CropOptions::overlayIcon(0), QStringLiteral("Crop guide overlay"));
    buildCropGridMenu(gridButton->menu(), gridButton, controller_, controller_ ? crop_ : nullptr);
    layout->addWidget(gridButton);
    // Settings (cog) menu: Classic mode, Show Cropped Area, Auto Center Preview
    // and the crop shield.
    auto* cogButton = menuIcon(QStringLiteral("optionsCropCog"), QStringLiteral("crop.cog"),
                               QStringLiteral("Crop settings"));
    if (controller_) {
        buildCropSettingsMenu(cogButton->menu(), controller_, crop_);
    }
    layout->addWidget(cogButton);
    layout->addWidget(separator());

    auto* deletePixels = new QCheckBox(QStringLiteral("Delete Cropped Pixels"), page);
    deletePixels->setObjectName(QStringLiteral("optionsCropDelete"));
    deletePixels->setChecked(controller_ ? crop_->deletePixels() : true);
    deletePixels->setToolTip(QStringLiteral(
        "Discard the pixels outside the crop rather than keeping them beyond the canvas edge"));
    layout->addWidget(deletePixels);
    layout->addWidget(separator());

    auto* reset = new QToolButton(page);
    reset->setObjectName(QStringLiteral("optionsCropReset"));
    reset->setIcon(icon(QStringLiteral("edit.undo")));
    reset->setIconSize(QSize(18, 18));
    reset->setToolTip(QStringLiteral("Reset the crop values"));
    layout->addWidget(reset);
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
                if (CropBoxSession* box = cropBox()) {
                    box->resizeCrop(width->value(), height->value());
                }
            } else {
                crop_->setRatio(
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
                if (CropBoxSession* box = cropBox()) {
                    box->resizeCrop(h, w);
                }
            } else {
                crop_->setRatio(w > 0.0 ? h / w : 0.0);
            }
            cropRatioW_ = h;
            cropRatioH_ = w;
            const QSignalBlocker bw(width);
            const QSignalBlocker bh(height);
            width->setValue(h);
            height->setValue(w);
        });
        const auto resetRatio = [this, ratio, width, height]() {
            cropRatioW_ = 0.0;
            cropRatioH_ = 0.0;
            crop_->setRatio(0.0);
            ratio->setCurrentIndex(0);
            const QSignalBlocker bw(width);
            const QSignalBlocker bh(height);
            width->setBlank(true);
            height->setBlank(true);
        };
        connect(clear, &QToolButton::clicked, this, resetRatio);
        connect(reset, &QToolButton::clicked, this, [this, resetRatio, spirit]() {
            resetRatio();
            if (CropBoxSession* box = cropBox()) {
                box->resetCropBox();
            }
            if (spirit->isChecked()) {
                spirit->setChecked(false);
            }
        });
        connect(spirit, &QToolButton::toggled, this,
                [this](bool on) { crop_->setStraightenMode(on); });
        connect(controller_, &ToolController::cropOptionsChanged, this, [this, spirit]() {
            const QSignalBlocker block(spirit);
            spirit->setChecked(crop_->straightenMode());
        });
        // The canvas context menu can toggle Delete Cropped Pixels; mirror it.
        connect(controller_, &ToolController::cropOptionsChanged, this, [this, deletePixels]() {
            const QSignalBlocker block(deletePixels);
            deletePixels->setChecked(crop_->deletePixels());
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
                        width->setValue(cropBox() ? cropBox()->cropWidth() : 0.0);
                        height->setValue(cropBox() ? cropBox()->cropHeight() : 0.0);
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
                    const bool constrained = pair.x() > 0.0 && pair.y() > 0.0;
                    cropRatioW_ = constrained ? pair.x() : 0.0;
                    cropRatioH_ = constrained ? pair.y() : 0.0;
                    const QSignalBlocker bw(width);
                    const QSignalBlocker bh(height);
                    width->setBlank(!constrained);
                    height->setBlank(!constrained);
                    if (constrained) {
                        width->setValue(pair.x());
                        height->setValue(pair.y());
                    }
                    crop_->setRatio(constrained ? pair.x() / pair.y() : 0.0);
                });
        connect(deletePixels, &QCheckBox::toggled, this,
                [this](bool on) { crop_->setDeletePixels(on); });
        connect(cancel, &QToolButton::clicked, this, [this]() { controller_->handleEscape(); });
        connect(commit, &QToolButton::clicked, this, [this]() {
            if (CropCommitSession* session = cropCommit()) {
                session->commitCrop();
            }
        });
        // Cancel/Apply/Reset are visible only while a box is active (not in the
        // preview or the no-box state after Escape). Re-synced on every crop-state
        // change, not just the tool switch.
        const auto syncCommitControls = [this, reset, cancel, commit]() {
            CropBoxSession* box = cropBox();
            const bool on = box && box->cropActive();
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
        cropWidthField_->setValue(cropBox() ? cropBox()->cropWidth() : 0.0);
        cropHeightField_->setValue(cropBox() ? cropBox()->cropHeight() : 0.0);
    } else if (cropRatioW_ > 0.0 && cropRatioH_ > 0.0) {
        cropWidthField_->setValue(cropRatioW_);
        cropHeightField_->setValue(cropRatioH_);
    } else {
        // Unconstrained: no ratio to show.
        cropWidthField_->setBlank(true);
        cropHeightField_->setBlank(true);
    }
}

// CS6's Perspective Crop bar: W/H output pixels (blank = auto), swap,
// Resolution with its unit, Front Image, Clear, and a Show Grid toggle, plus
// Cancel/Apply shown only while the quad is complete.
QWidget* OptionsBar::buildPerspectiveCropPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    const auto separator = [page]() {
        auto* line = new QFrame(page);
        line->setObjectName(QStringLiteral("optionsToolSeparator"));
        line->setFrameShape(QFrame::VLine);
        line->setFrameShadow(QFrame::Plain);
        line->setFixedWidth(1);
        line->setFixedHeight(18);
        return line;
    };
    layout->addWidget(toolButton(id, page));

    const auto pixelField = [page](const QString& name) {
        return new NumericField(
            QString(),
            numericConfig(1.0, 100000.0, 1.0, 0, QStringLiteral(" px"), false, name, true), page);
    };
    auto* width = pixelField(QStringLiteral("optionsPerspectiveWidth"));
    width->setBlank(true);
    layout->addWidget(width);
    auto* swap = new QToolButton(page);
    swap->setObjectName(QStringLiteral("optionsPerspectiveSwap"));
    swap->setIcon(icon(QStringLiteral("crop.swap")));
    swap->setIconSize(QSize(18, 18));
    swap->setToolTip(QStringLiteral("Swap the output width and height"));
    layout->addWidget(swap);
    auto* height = pixelField(QStringLiteral("optionsPerspectiveHeight"));
    height->setBlank(true);
    layout->addWidget(height);
    layout->addWidget(separator());

    layout->addWidget(new QLabel(QStringLiteral("Resolution:"), page));
    auto* resolution = new NumericField(
        QString(),
        numericConfig(1.0, 100000.0, 1.0, 0, QString(), false,
                      QStringLiteral("optionsPerspectiveResolution")),
        page);
    layout->addWidget(resolution);
    auto* unit = new QComboBox(page);
    unit->setObjectName(QStringLiteral("optionsPerspectiveUnit"));
    unit->addItems({QStringLiteral("Pixels/in"), QStringLiteral("Pixels/cm")});
    unit->setSizeAdjustPolicy(QComboBox::AdjustToContents);
    layout->addWidget(unit);
    layout->addWidget(separator());

    const auto textButton = [page](const QString& name, const QString& text) {
        auto* button = new QToolButton(page);
        button->setObjectName(name);
        button->setText(text);
        return button;
    };
    auto* front = textButton(QStringLiteral("optionsPerspectiveFrontImage"),
                             QStringLiteral("Front Image"));
    layout->addWidget(front);
    auto* clear = textButton(QStringLiteral("optionsPerspectiveClear"), QStringLiteral("Clear"));
    clear->setToolTip(QStringLiteral("Clear the output size fields"));
    layout->addWidget(clear);
    auto* showGrid =
        textButton(QStringLiteral("optionsPerspectiveShowGrid"), QStringLiteral("Show Grid"));
    showGrid->setCheckable(true);
    showGrid->setChecked(true);
    layout->addWidget(showGrid);
    layout->addWidget(separator());

    auto* reset = new QToolButton(page);
    reset->setObjectName(QStringLiteral("optionsPerspectiveReset"));
    reset->setIcon(icon(QStringLiteral("edit.undo")));
    reset->setIconSize(QSize(18, 18));
    reset->setToolTip(QStringLiteral("Reset the perspective crop"));
    layout->addWidget(reset);
    auto* cancel = new QToolButton(page);
    cancel->setObjectName(QStringLiteral("optionsPerspectiveCancel"));
    cancel->setIcon(icon(QStringLiteral("crop.cancel")));
    cancel->setIconSize(QSize(18, 18));
    cancel->setToolTip(QStringLiteral("Cancel the perspective crop (Esc)"));
    layout->addWidget(cancel);
    auto* apply = new QToolButton(page);
    apply->setObjectName(QStringLiteral("optionsPerspectiveApply"));
    apply->setIcon(icon(QStringLiteral("crop.apply")));
    apply->setIconSize(QSize(18, 18));
    apply->setToolTip(QStringLiteral("Apply the perspective crop (Enter)"));
    layout->addWidget(apply);

    const auto setResolutionFields = [resolution, unit](bool on) {
        resolution->setVisible(on);
        unit->setVisible(on);
    };
    setResolutionFields(true);

    if (controller_) {
        const auto pushOutput = [this, width, height]() {
            if (PerspectiveCropSession* perspective = perspectiveCrop()) {
                perspective->setPerspectiveOutput(
                    width->isBlank() ? 0 : int(std::lround(width->value())),
                    height->isBlank() ? 0 : int(std::lround(height->value())));
            }
        };
        connect(width, &NumericField::valueChanged, this, [pushOutput](double) { pushOutput(); });
        connect(height, &NumericField::valueChanged, this, [pushOutput](double) { pushOutput(); });
        connect(swap, &QToolButton::clicked, this, [width, height, pushOutput]() {
            const bool wb = width->isBlank();
            const bool hb = height->isBlank();
            const double w = width->value();
            const double h = height->value();
            width->setBlank(hb);
            height->setBlank(wb);
            if (!hb) width->setValue(h);
            if (!wb) height->setValue(w);
            pushOutput();
        });
        connect(front, &QToolButton::clicked, this, [this, resolution]() {
            if (PictureView* v = controller_->view(); v && v->has_document()) {
                const double ppi = document_ppi(*v);
                if (ppi > 0.0) resolution->setValue(ppi);
            }
        });
        connect(clear, &QToolButton::clicked, this, [width, height, pushOutput]() {
            width->setBlank(true);
            height->setBlank(true);
            pushOutput();
        });
        connect(showGrid, &QToolButton::toggled, this, [this](bool on) {
            if (PerspectiveCropSession* perspective = perspectiveCrop()) {
                perspective->setPerspectiveShowGrid(on);
            }
        });
        connect(reset, &QToolButton::clicked, this, [this, width, height, pushOutput]() {
            width->setBlank(true);
            height->setBlank(true);
            pushOutput();
            if (PerspectiveCropSession* perspective = perspectiveCrop()) {
                perspective->resetPerspectiveQuad();
            }
        });
        connect(cancel, &QToolButton::clicked, this, [this]() { controller_->handleEscape(); });
        connect(apply, &QToolButton::clicked, this, [this]() {
            if (CropCommitSession* session = cropCommit()) {
                session->commitCrop();
            }
        });
        perspectiveReset_ = reset;
        perspectiveApply_ = apply;
        perspectiveCancel_ = cancel;
        syncPerspectiveButtons();
        connect(controller_, &ToolController::toolSessionChanged, this,
                [this]() { syncPerspectiveButtons(); });
    }
    layout->addStretch(1);
    return page;
}

void OptionsBar::syncPerspectiveButtons()
{
    if (!perspectiveApply_ || !perspectiveCancel_ || !perspectiveReset_) {
        return;
    }
    PerspectiveCropSession* perspective = perspectiveCrop();
    const bool on = perspective && perspective->perspectiveQuadComplete();
    perspectiveReset_->setVisible(on);
    perspectiveApply_->setVisible(on);
    perspectiveCancel_->setVisible(on);
}

} // namespace pictura
