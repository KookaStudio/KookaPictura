#include "properties_panel.h"
#include "api/type_api.h"

#include "adjustment_controls.h"
#include "api/transform_api.h"
#include "icons.h"
#include "layers_panel_internal.h"
#include "numeric_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/adjustment_edit.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_refine.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"

#include <QtCore/QTimer>
#include <QtGui/QPainter>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <map>

namespace pictura {

namespace {

// A pause this long after the last live edit (a wheel, arrow keys, or a curve
// drag, none of which has a release to hang the commit on) ends the gesture.
constexpr int kCommitDelayMs = 500;

enum Page { NothingPage, AdjustmentPage, LayerPage };

// CS6's canvas ceiling (Canvas Size / New).
constexpr int kMaxCanvas = 300000;

struct ModeEntry {
    const char* key;
    const char* name;
};

// Image ▸ Mode's order; the keys are `image_mode`'s names.
constexpr ModeEntry kModes[] = {
    {"bitmap", "Bitmap"},       {"grayscale", "Grayscale"}, {"duotone", "Duotone"},
    {"indexed", "Indexed Color"}, {"rgb", "RGB Color"},     {"cmyk", "CMYK Color"},
    {"lab", "Lab Color"},       {"multichannel", "Multichannel"},
};

QSpinBox* canvasSpin(QWidget* parent, const QString& name)
{
    auto* spin = new QSpinBox(parent);
    spin->setObjectName(name);
    spin->setRange(1, kMaxCanvas);
    spin->setSuffix(QStringLiteral(" px"));
    spin->setKeyboardTracking(false);
    return spin;
}

void setItemEnabled(QComboBox* combo, int index, bool enabled)
{
    if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
        model->item(index)->setEnabled(enabled);
    }
}

QToolButton* footerButton(QWidget* parent, const QString& iconId, const QString& tip,
                          const QString& name)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(name);
    button->setIcon(icon(iconId));
    button->setToolTip(tip);
    button->setAutoRaise(true);
    return button;
}

// Mask Density as a percentage over the PSD's 0-255 byte.
NumericField* densityField(QWidget* parent, const QString& name)
{
    return new NumericField(QStringLiteral("Density"),
                            NumericFieldConfig{0.0, 100.0, 1.0, 10.0, 0, QStringLiteral("%"), true,
                                               name, name},
                            parent);
}

// Mask Feather in pixels, to the selection feather's 250 px maximum.
NumericField* featherField(QWidget* parent, const QString& name)
{
    return new NumericField(QStringLiteral("Feather"),
                            NumericFieldConfig{0.0, 250.0, 0.1, 1.0, 1, QStringLiteral(" px"),
                                               true, name, name},
                            parent);
}

double densityPercent(int byte)
{
    return qRound(byte * 100.0 / 255.0);
}

// The byte for a whole percent; `current` when it already shows as `percent`,
// so committing an untouched field (76 shows 30 %, which maps to 77) is a no-op.
int densityByte(int current, double percent)
{
    if (current >= 0 && densityPercent(current) == percent) {
        return current;
    }
    return qRound(percent * 255.0 / 100.0);
}

} // namespace

PropertiesPanel::PropertiesPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);
    header_ = new QLabel(this);
    header_->setObjectName(QStringLiteral("propertiesHeader"));
    QFont bold = header_->font();
    bold.setBold(true);
    header_->setFont(bold);
    layout->addWidget(header_);
    // One scroll area for every page, the footer outside it: the panel's
    // minimum size stays small, so it never crowds the panel column.
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Ignored);
    auto* body = new QWidget(scroll);
    auto* bodyLayout = new QVBoxLayout(body);
    bodyLayout->setContentsMargins(0, 0, 0, 0);
    stack_ = new QStackedWidget(body);
    bodyLayout->addWidget(stack_);
    buildCanvas(bodyLayout);
    bodyLayout->addStretch(1);
    scroll->setWidget(body);
    layout->addWidget(scroll, 1);

    auto* nothing = new QLabel(QStringLiteral("No Properties"), stack_);
    nothing->setAlignment(Qt::AlignCenter);
    nothing->setEnabled(false);
    stack_->addWidget(nothing);

    controls_ = new AdjustmentControls(stack_);
    controls_->setObjectName(QStringLiteral("propertiesControls"));
    controls_->setCurveSource([this](int channel) {
        return view_ ? QString(adjustment_curve(*view_, path_, channel)) : QString();
    });
    connect(controls_, &AdjustmentControls::valueChanged, this, &PropertiesPanel::push);
    connect(controls_, &AdjustmentControls::curveChanged, this,
            [this](int channel, const QString& points) {
                if (view_ && adjustment_set_curve(*view_, path_, channel, points)) {
                    edited();
                }
            });
    connect(controls_, &AdjustmentControls::gestureEnded, this, &PropertiesPanel::commit);
    stack_->addWidget(controls_);

    // CS6's footer: Clip to Layer, Reset, Toggle Visibility, Delete.
    footer_ = new QWidget(this);
    auto* footer = new QHBoxLayout(footer_);
    footer->setContentsMargins(0, 0, 0, 0);
    clip_ = footerButton(footer_, QStringLiteral("layers.clipMask"),
                         QStringLiteral("This adjustment clips to the layer (click to affect "
                                        "all layers below)"),
                         QStringLiteral("propertiesClip"));
    clip_->setCheckable(true);
    auto* reset = footerButton(footer_, QStringLiteral("edit.undo"),
                               QStringLiteral("Reset to adjustment defaults"),
                               QStringLiteral("propertiesReset"));
    visible_ = footerButton(footer_, QStringLiteral("layers.eyeOn"),
                            QStringLiteral("Toggle layer visibility"),
                            QStringLiteral("propertiesVisible"));
    auto* remove = footerButton(footer_, QStringLiteral("layers.delete"),
                                QStringLiteral("Delete this adjustment layer"),
                                QStringLiteral("propertiesDelete"));
    footer->addStretch(1);
    for (QToolButton* button : {clip_, reset, visible_, remove}) {
        footer->addWidget(button);
    }
    layout->addWidget(footer_);

    layerPage_ = new QWidget(stack_);
    auto* layerPageLayout = new QVBoxLayout(layerPage_);
    layerPageLayout->setContentsMargins(0, 0, 0, 0);
    info_ = new QFormLayout();
    layerPageLayout->addLayout(info_);

    // Mask section: shown only when the active layer carries a layer mask. The
    // parameter rows are placeholders until the model stores them.
    maskSection_ = new QWidget(layerPage_);
    auto* maskLayout = new QVBoxLayout(maskSection_);
    maskLayout->setContentsMargins(0, 8, 0, 0);
    auto* maskTitle = new QLabel(QStringLiteral("Mask"), maskSection_);
    QFont maskBold = maskTitle->font();
    maskBold.setBold(true);
    maskTitle->setFont(maskBold);
    maskLayout->addWidget(maskTitle);
    maskLayout->addWidget(new QLabel(QStringLiteral("Layer Mask"), maskSection_));
    auto* maskButtons = new QHBoxLayout();
    const auto maskButton = [this, maskButtons](const QString& iconId, const QString& tip,
                                                const QString& name) {
        QToolButton* button = footerButton(maskSection_, iconId, tip, name);
        maskButtons->addWidget(button);
        return button;
    };
    maskEnable_ = maskButton(QStringLiteral("layers.eyeOn"), QStringLiteral("Enable the layer mask"),
                             QStringLiteral("propertiesMaskEnable"));
    maskDisable_ = maskButton(QStringLiteral("layers.eyeOff"),
                              QStringLiteral("Disable the layer mask"),
                              QStringLiteral("propertiesMaskDisable"));
    maskLink_ = maskButton(QStringLiteral("layers.link"), QStringLiteral("Link the layer mask"),
                           QStringLiteral("propertiesMaskLink"));
    maskUnlink_ = maskButton(QStringLiteral("layers.link"), QStringLiteral("Unlink the layer mask"),
                             QStringLiteral("propertiesMaskUnlink"));
    maskDelete_ = maskButton(QStringLiteral("layers.delete"),
                             QStringLiteral("Delete the layer mask"),
                             QStringLiteral("propertiesMaskDelete"));
    maskApply_ = maskButton(QStringLiteral("layers.mask"), QStringLiteral("Apply the layer mask"),
                            QStringLiteral("propertiesMaskApply"));
    maskButtons->addStretch(1);
    maskLayout->addLayout(maskButtons);
    maskDensity_ = densityField(maskSection_, QStringLiteral("propertiesMaskDensity"));
    maskFeather_ = featherField(maskSection_, QStringLiteral("propertiesMaskFeather"));
    maskLayout->addWidget(maskDensity_);
    maskLayout->addWidget(maskFeather_);
    maskInvert_ = new QToolButton(maskSection_);
    maskInvert_->setText(QStringLiteral("Invert"));
    maskInvert_->setToolTip(QStringLiteral("Invert the layer mask"));
    maskInvert_->setObjectName(QStringLiteral("propertiesMaskInvert"));
    maskLayout->addWidget(maskInvert_);
    maskSection_->hide();
    layerPageLayout->addWidget(maskSection_);

    // Vector Mask section: shown only when the active layer carries a vector
    // mask.
    vectorMaskSection_ = new QWidget(layerPage_);
    auto* vectorLayout = new QVBoxLayout(vectorMaskSection_);
    vectorLayout->setContentsMargins(0, 8, 0, 0);
    auto* vectorTitle = new QLabel(QStringLiteral("Vector Mask"), vectorMaskSection_);
    QFont vectorBold = vectorTitle->font();
    vectorBold.setBold(true);
    vectorTitle->setFont(vectorBold);
    vectorLayout->addWidget(vectorTitle);
    auto* vectorButtons = new QHBoxLayout();
    const auto vectorButton = [this, vectorButtons](const QString& iconId, const QString& tip,
                                                    const QString& name) {
        QToolButton* button = footerButton(vectorMaskSection_, iconId, tip, name);
        vectorButtons->addWidget(button);
        return button;
    };
    vectorMaskEnable_ = vectorButton(QStringLiteral("layers.eyeOn"),
                                     QStringLiteral("Enable the vector mask"),
                                     QStringLiteral("propertiesVectorMaskEnable"));
    vectorMaskDisable_ = vectorButton(QStringLiteral("layers.eyeOff"),
                                      QStringLiteral("Disable the vector mask"),
                                      QStringLiteral("propertiesVectorMaskDisable"));
    vectorMaskLink_ = vectorButton(QStringLiteral("layers.link"),
                                   QStringLiteral("Link the vector mask"),
                                   QStringLiteral("propertiesVectorMaskLink"));
    vectorMaskUnlink_ = vectorButton(QStringLiteral("layers.link"),
                                     QStringLiteral("Unlink the vector mask"),
                                     QStringLiteral("propertiesVectorMaskUnlink"));
    vectorMaskDelete_ = vectorButton(QStringLiteral("layers.delete"),
                                     QStringLiteral("Delete the vector mask"),
                                     QStringLiteral("propertiesVectorMaskDelete"));
    vectorMaskRasterize_ = vectorButton(QStringLiteral("layers.mask"),
                                        QStringLiteral("Rasterize the vector mask"),
                                        QStringLiteral("propertiesVectorMaskRasterize"));
    vectorButtons->addStretch(1);
    vectorLayout->addLayout(vectorButtons);
    vectorMaskDensity_ =
        densityField(vectorMaskSection_, QStringLiteral("propertiesVectorMaskDensity"));
    vectorMaskFeather_ =
        featherField(vectorMaskSection_, QStringLiteral("propertiesVectorMaskFeather"));
    vectorLayout->addWidget(vectorMaskDensity_);
    vectorLayout->addWidget(vectorMaskFeather_);
    vectorMaskSection_->hide();
    layerPageLayout->addWidget(vectorMaskSection_);

    stack_->addWidget(layerPage_);

    connect(maskEnable_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_set_enabled(*view_, true)) {
            refresh();
        }
    });
    connect(maskDisable_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_set_enabled(*view_, false)) {
            refresh();
        }
    });
    connect(maskLink_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_set_linked(*view_, true)) {
            refresh();
        }
    });
    connect(maskUnlink_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_set_linked(*view_, false)) {
            refresh();
        }
    });
    connect(maskDelete_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_delete(*view_)) {
            refresh();
        }
    });
    connect(maskApply_, &QToolButton::clicked, this, [this]() {
        if (view_ && layer_mask_apply(*view_)) {
            refresh();
        }
    });

    // Dragging previews; the finished edit records one history state.
    const auto wireRefinement = [this](NumericField* field, auto set) {
        connect(field, &NumericField::valueChanged, this, [this, set](double v) {
            if (view_) {
                set(*view_, v, false);
            }
        });
        connect(field, &NumericField::valueCommitted, this, [this, set](double v) {
            if (view_) {
                set(*view_, v, true);
            }
        });
    };
    wireRefinement(maskDensity_, [](PictureView& view, double percent, bool commit) {
        return layer_mask_set_density(view, densityByte(layer_mask_density(view), percent),
                                      commit);
    });
    wireRefinement(maskFeather_, &layer_mask_set_feather);
    wireRefinement(vectorMaskDensity_, [](PictureView& view, double percent, bool commit) {
        return vector_mask_set_density(view, densityByte(vector_mask_density(view), percent),
                                       commit);
    });
    wireRefinement(vectorMaskFeather_, &vector_mask_set_feather);
    connect(maskInvert_, &QToolButton::clicked, this, [this]() {
        if (view_) {
            layer_mask_invert(*view_);
        }
    });

    connect(vectorMaskEnable_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_set_enabled(*view_, true)) {
            refresh();
        }
    });
    connect(vectorMaskDisable_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_set_enabled(*view_, false)) {
            refresh();
        }
    });
    connect(vectorMaskLink_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_set_linked(*view_, true)) {
            refresh();
        }
    });
    connect(vectorMaskUnlink_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_set_linked(*view_, false)) {
            refresh();
        }
    });
    connect(vectorMaskDelete_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_delete(*view_)) {
            refresh();
        }
    });
    connect(vectorMaskRasterize_, &QToolButton::clicked, this, [this]() {
        if (view_ && vector_mask_rasterize(*view_)) {
            refresh();
        }
    });

    commitTimer_ = new QTimer(this);
    commitTimer_->setSingleShot(true);
    commitTimer_->setInterval(kCommitDelayMs);
    connect(commitTimer_, &QTimer::timeout, this, &PropertiesPanel::commit);

    connect(clip_, &QToolButton::clicked, this, [this]() {
        commit();
        if (view_) {
            clipping_toggle(*view_, path_);
        }
        refresh();
    });
    connect(reset, &QToolButton::clicked, this, [this]() {
        commit();
        // The controls stay; only their values reload.
        if (view_ && adjustment_reset(*view_, path_)) {
            dirty_ = true;
            commit();
            refresh();
        }
    });
    connect(visible_, &QToolButton::clicked, this, [this]() {
        commit();
        if (view_) {
            const auto row = pictura::layers_api::rowForPath(*view_, path_);
            if (row.path == path_) {
                view_->set_layers_visible(QStringList{path_}, !row.visible);
            }
        }
        refresh();
    });
    connect(remove, &QToolButton::clicked, this, [this]() {
        commit();
        if (view_) {
            view_->delete_layers(QStringList{path_});
        }
        refresh();
    });
    refresh();
}

void PropertiesPanel::buildCanvas(QVBoxLayout* layout)
{
    canvas_ = new QWidget(this);
    canvas_->setObjectName(QStringLiteral("propertiesCanvas"));
    auto* grid = new QGridLayout(canvas_);
    grid->setContentsMargins(0, 8, 0, 0);
    auto* title = new QLabel(QStringLiteral("Canvas"), canvas_);
    QFont bold = title->font();
    bold.setBold(true);
    title->setFont(bold);
    grid->addWidget(title, 0, 0, 1, 3);

    link_ = new QToolButton(canvas_);
    link_->setObjectName(QStringLiteral("propertiesCanvasLink"));
    link_->setIcon(icon(QStringLiteral("layers.link")));
    link_->setToolTip(QStringLiteral("Link width and height"));
    link_->setCheckable(true);
    link_->setAutoRaise(true);
    width_ = canvasSpin(canvas_, QStringLiteral("propertiesCanvasWidth"));
    height_ = canvasSpin(canvas_, QStringLiteral("propertiesCanvasHeight"));
    grid->addWidget(link_, 1, 0, 2, 1, Qt::AlignVCenter);
    grid->addWidget(new QLabel(QStringLiteral("W"), canvas_), 1, 1);
    grid->addWidget(width_, 1, 2);
    grid->addWidget(new QLabel(QStringLiteral("H"), canvas_), 2, 1);
    grid->addWidget(height_, 2, 2);

    resolution_ = new QLabel(canvas_);
    resolution_->setObjectName(QStringLiteral("propertiesCanvasResolution"));
    grid->addWidget(resolution_, 3, 0, 1, 3, Qt::AlignHCenter);

    mode_ = new QComboBox(canvas_);
    mode_->setObjectName(QStringLiteral("propertiesCanvasMode"));
    for (const ModeEntry& entry : kModes) {
        mode_->addItem(QLatin1String(entry.name), QLatin1String(entry.key));
    }
    depth_ = new QComboBox(canvas_);
    depth_->setObjectName(QStringLiteral("propertiesCanvasDepth"));
    for (int bits : {1, 8, 16, 32}) {
        depth_->addItem(bits == 1 ? QStringLiteral("1 Bit/Channel")
                                  : QStringLiteral("%1 Bits/Channel").arg(bits),
                        bits);
    }
    grid->addWidget(new QLabel(QStringLiteral("Mode"), canvas_), 4, 0, 1, 2);
    grid->addWidget(mode_, 4, 2);
    grid->addWidget(depth_, 5, 2);
    grid->setColumnStretch(2, 1);
    layout->addWidget(canvas_);

    // A linked edit keeps the document's aspect; the other field follows as
    // the user types, and Enter or leaving the field resizes.
    connect(width_, &QSpinBox::valueChanged, this, [this](int w) {
        if (link_->isChecked() && view_ && view_->document_width() > 0) {
            const QSignalBlocker block(height_);
            height_->setValue(std::max(
                1, qRound(double(w) * view_->document_height() / view_->document_width())));
        }
    });
    connect(height_, &QSpinBox::valueChanged, this, [this](int h) {
        if (link_->isChecked() && view_ && view_->document_height() > 0) {
            const QSignalBlocker block(width_);
            width_->setValue(std::max(
                1, qRound(double(h) * view_->document_width() / view_->document_height())));
        }
    });
    connect(width_, &QSpinBox::editingFinished, this, &PropertiesPanel::applyCanvasSize);
    connect(height_, &QSpinBox::editingFinished, this, &PropertiesPanel::applyCanvasSize);
    connect(mode_, &QComboBox::activated, this,
            [this](int index) { emit modeRequested(mode_->itemData(index).toString()); });
    connect(depth_, &QComboBox::activated, this,
            [this](int index) { emit depthRequested(depth_->itemData(index).toInt()); });
}

void PropertiesPanel::applyCanvasSize()
{
    if (!view_ || !view_->has_document()) {
        return;
    }
    const int w = width_->value();
    const int h = height_->value();
    if (w == view_->document_width() && h == view_->document_height()) {
        return;
    }
    view_->resize_canvas(QStringLiteral("center"), w, h);
    refresh();
}

void PropertiesPanel::refreshCanvas()
{
    // A field being typed into keeps its text until the edit finishes.
    if (!width_->hasFocus()) {
        const QSignalBlocker block(width_);
        width_->setValue(view_->document_width());
    }
    if (!height_->hasFocus()) {
        const QSignalBlocker block(height_);
        height_->setValue(view_->document_height());
    }
    resolution_->setText(QStringLiteral("Resolution: %1").arg(image_resolution(*view_)));

    const QString mode = image_mode(*view_);
    for (int i = 0; i < mode_->count(); ++i) {
        const QString key = mode_->itemData(i).toString();
        setItemEnabled(mode_, i, key == mode || image_mode_available(*view_, key));
        if (key == mode) {
            mode_->setCurrentIndex(i);
        }
    }
    const int bits = image_depth_bits(*view_);
    for (int i = 0; i < depth_->count(); ++i) {
        const int value = depth_->itemData(i).toInt();
        setItemEnabled(depth_, i, value == bits || image_depth_available(*view_, value));
        if (value == bits) {
            depth_->setCurrentIndex(i);
        }
    }
    canvas_->show();
}

void PropertiesPanel::setPage(int index)
{
    for (int i = 0; i < stack_->count(); ++i) {
        stack_->widget(i)->setSizePolicy(QSizePolicy::Preferred, i == index
                                                                     ? QSizePolicy::Preferred
                                                                     : QSizePolicy::Ignored);
    }
    stack_->setCurrentIndex(index);
    // The body is stack, Canvas, stretch: the layer summary and Canvas sit on
    // top; No Properties and an adjustment's controls take the whole panel.
    if (auto* body = qobject_cast<QBoxLayout*>(stack_->parentWidget()->layout())) {
        body->setStretch(0, index == LayerPage ? 0 : 1);
        body->setStretch(2, index == LayerPage ? 1 : 0);
    }
}

QString PropertiesPanel::resolutionForTest() const { return resolution_->text(); }

bool PropertiesPanel::maskSectionVisibleForTest() const
{
    return maskSection_ && maskSection_->isVisibleTo(this);
}

bool PropertiesPanel::vectorMaskSectionVisibleForTest() const
{
    return vectorMaskSection_ && vectorMaskSection_->isVisibleTo(this);
}

void PropertiesPanel::setView(PictureView* view)
{
    if (view != view_) {
        commit();
    }
    view_ = view;
    refresh();
}

QString PropertiesPanel::activePath() const
{
    if (!view_ || !view_->has_document()) {
        return QString();
    }
    const QString active = view_->active_layer_path();
    if (!active.isEmpty()
        && pictura::layers_api::rowForPath(*view_, active).path == active) {
        return active;
    }
    return QString();
}

void PropertiesPanel::refresh()
{
    const QString path = activePath();
    if (path != path_) {
        commit();
    }
    path_ = path;
    if (path_.isEmpty()) {
        showNothing();
        return;
    }
    const pictura::layers_api::LayerRow row = pictura::layers_api::rowForPath(*view_, path_);
    if (row.path != path_) {
        showNothing();
        return;
    }
    const QStringList page = adjustment_page(*view_, path_);
    if (page.isEmpty()) {
        showLayer(row);
        return;
    }
    clip_->setChecked(row.clipping);
    visible_->setIcon(icon(row.visible ? QStringLiteral("layers.eyeOn")
                                       : QStringLiteral("layers.eyeOff")));
    showAdjustment(page);
}

void PropertiesPanel::showNothing()
{
    footer_->hide();
    if (view_ && view_->has_document()) {
        header_->setText(QStringLiteral("Document"));
        setPage(LayerPage);
        stack_->hide();
        refreshCanvas();
        return;
    }
    header_->setText(QStringLiteral("No Properties"));
    stack_->show();
    setPage(NothingPage);
    canvas_->hide();
}

void PropertiesPanel::showLayer(const pictura::layers_api::LayerRow& row)
{
    header_->setText(QStringLiteral("Layer Properties"));
    stack_->show();
    setPage(LayerPage);
    footer_->hide();
    refreshCanvas();
    while (info_->rowCount() > 0) {
        info_->removeRow(0);
    }
    const QString kind = pictura::type_api::isType(*view_, path_) ? QStringLiteral("type") : row.kind;
    const std::map<QString, QString> kinds = {
        {QStringLiteral("pixel"), QStringLiteral("Pixel")},
        {QStringLiteral("type"), QStringLiteral("Type")},
        {QStringLiteral("group"), QStringLiteral("Group")},
        {QStringLiteral("background"), QStringLiteral("Background")},
        {QStringLiteral("adjustment"), QStringLiteral("Adjustment")}};
    const auto it = kinds.find(kind);
    info_->addRow(QStringLiteral("Kind:"),
                  new QLabel(it != kinds.end() ? it->second : kind, layerPage_));
    const transform_api::LayerRect rect = transform_api::layerRect(*view_, path_);
    if (rect.valid && kind != QStringLiteral("group")) {
        info_->addRow(QStringLiteral("Size:"),
                      new QLabel(QStringLiteral("%1 x %2 px")
                                     .arg(rect.width())
                                     .arg(rect.height()),
                                 layerPage_));
        info_->addRow(QStringLiteral("Position:"),
                      new QLabel(QStringLiteral("X %1, Y %2").arg(rect.left).arg(rect.top),
                                 layerPage_));
    }
    QString blend = row.blend;
    for (const BlendEntry& entry : kBlends) {
        if (blend == QLatin1String(entry.key)) {
            blend = QLatin1String(entry.name);
        }
    }
    info_->addRow(QStringLiteral("Blend Mode:"), new QLabel(blend, layerPage_));
    const auto percent = [](int v) { return QStringLiteral("%1%").arg(qRound(v * 100.0 / 255.0)); };
    info_->addRow(QStringLiteral("Opacity:"),
                  new QLabel(percent(row.opacity), layerPage_));
    info_->addRow(QStringLiteral("Fill:"), new QLabel(percent(row.fill), layerPage_));
    info_->addRow(QStringLiteral("Mask:"),
                  new QLabel(row.hasMask ? QStringLiteral("Layer Mask")
                                         : QStringLiteral("None"),
                             layerPage_));
    const int lock = row.lockBits;
    QStringList locks;
    if (lock & 0x01) {
        locks << QStringLiteral("Transparency");
    }
    if (lock & 0x02) {
        locks << QStringLiteral("Pixels");
    }
    if (lock & 0x04) {
        locks << QStringLiteral("Position");
    }
    info_->addRow(QStringLiteral("Locks:"),
                  new QLabel(locks.isEmpty() ? QStringLiteral("None") : locks.join(QStringLiteral(", ")),
                             layerPage_));
    maskSection_->setVisible(row.hasMask);
    vectorMaskSection_->setVisible(row.hasVectorMask);
    if (row.hasMask) {
        maskDensity_->setValue(densityPercent(layer_mask_density(*view_)));
        maskFeather_->setValue(layer_mask_feather(*view_));
    }
    if (row.hasVectorMask) {
        vectorMaskDensity_->setValue(densityPercent(vector_mask_density(*view_)));
        vectorMaskFeather_->setValue(vector_mask_feather(*view_));
    }
}

void PropertiesPanel::showAdjustment(const QStringList& page)
{
    stack_->show();
    setPage(AdjustmentPage);
    canvas_->hide();
    footer_->show();
    controls_->setPage(page, dirty_);
    title_ = controls_->title();
    header_->setText(title_);
}

void PropertiesPanel::push(const QString& key, double value)
{
    if (!view_ || path_.isEmpty()) {
        return;
    }
    if (adjustment_set(*view_, path_, key, value)) {
        edited();
    }
}

void PropertiesPanel::edited()
{
    dirty_ = true;
    commitTimer_->start();
}

void PropertiesPanel::commit()
{
    commitTimer_->stop();
    if (!dirty_) {
        return;
    }
    dirty_ = false;
    if (view_) {
        adjustment_commit(*view_, title_);
    }
}

QString PropertiesPanel::messageForTest() const { return header_->text(); }

QWidget* PropertiesPanel::controlForTest(const QString& key) const
{
    return controls_->controlForTest(key);
}

} // namespace pictura
