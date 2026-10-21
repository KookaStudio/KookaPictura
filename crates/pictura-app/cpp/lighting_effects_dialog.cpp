#include "lighting_effects_dialog.h"
#include "color_picker_dialog.h"
#include "dialogs.h"
#include "icons.h"
#include "lighting_canvas.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtCore/QtMath>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "panels/color_swatch_widget.h"

namespace pictura {

namespace {

// The workspace renders a proxy this size at most; the rig is size-invariant,
// so it is the same picture OK commits.
const int kProxyEdge = 1100;
const int kColumnWidth = 264;
const QString kCustom = QStringLiteral("Custom");

// The Lights panel's and add buttons' glyphs: a spotlight's cone, a bulb, and
// the sun, as CS6 draws them.
QIcon lightGlyph(LightKind kind)
{
    QPixmap pixmap(32, 32);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QColor ink(0xd0, 0xd0, 0xd0);
    painter.setPen(QPen(ink, 2.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    switch (kind) {
    case LightKind::Spot: {
        QPainterPath cone;
        cone.moveTo(6, 10);
        cone.lineTo(18, 6);
        cone.lineTo(18, 20);
        cone.closeSubpath();
        painter.setBrush(ink);
        painter.drawPath(cone);
        painter.setBrush(Qt::NoBrush);
        painter.drawLine(QPointF(21, 9), QPointF(27, 6));
        painter.drawLine(QPointF(21, 14), QPointF(28, 15));
        painter.drawLine(QPointF(21, 19), QPointF(26, 24));
        break;
    }
    case LightKind::Point:
        painter.setBrush(ink);
        painter.drawEllipse(QPointF(16, 12), 7, 7);
        painter.drawRect(QRectF(13, 20, 6, 6));
        break;
    case LightKind::Infinite:
        painter.setBrush(ink);
        painter.drawEllipse(QPointF(16, 16), 5, 5);
        for (int i = 0; i < 8; ++i) {
            const double a = qDegreesToRadians(i * 45.0);
            painter.drawLine(QPointF(16 + 9 * std::cos(a), 16 + 9 * std::sin(a)),
                             QPointF(16 + 13 * std::cos(a), 16 + 13 * std::sin(a)));
        }
        break;
    }
    painter.end();
    return QIcon(pixmap);
}

void paintSwatch(ColorSwatchWidget* swatch, const QColor& color)
{
    swatch->setToolTip(color.name());
    swatch->setColor(color);
}

QLabel* panelTab(const QString& title, QWidget* parent)
{
    auto* tab = new QLabel(title, parent);
    tab->setObjectName(QStringLiteral("sectionTabLabel"));
    return tab;
}

} // namespace

void LightingEffectsDialog::Field::setValue(double value)
{
    const QSignalBlocker blockSpin(spin);
    const QSignalBlocker blockSlider(slider);
    spin->setValue(qRound(value));
    slider->setValue(spin->value());
}

void LightingEffectsDialog::Field::setEnabled(bool on)
{
    label->setEnabled(on);
    spin->setEnabled(on);
    slider->setEnabled(on);
}

LightingEffectsDialog::LightingEffectsDialog(PictureView* view, const FilterCommandSpec& spec,
                                             QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec), rig_(defaultLightingRig())
{
    setWindowTitle(spec.label);
    setObjectName(QStringLiteral("lightingEffectsWorkspace"));
    setMinimumSize(820, 560);
    const QWidget* frame = parent ? parent->window() : nullptr;
    resize(frame ? (QSizeF(frame->size()) * 0.92).toSize().expandedTo(minimumSize())
                 : QSize(1180, 760));

    canvas_ = new LightingCanvas(&rig_, this);
    if (view_ && view_->has_document()) {
        const QImage image = documentPreviewImage(view_, kProxyEdge);
        if (!image.isNull()) {
            proxy_ = (image.width() > kProxyEdge || image.height() > kProxyEdge
                          ? image.scaled(kProxyEdge, kProxyEdge, Qt::KeepAspectRatio,
                                         Qt::SmoothTransformation)
                          : image)
                         .convertToFormat(QImage::Format_RGBA8888);
            for (int y = 0; y < proxy_.height(); ++y) {
                proxyRgba_.append(reinterpret_cast<const char*>(proxy_.constScanLine(y)),
                                  proxy_.width() * 4);
            }
            canvas_->setAspect(QSize(view_->document_width(), view_->document_height()));
        }
    }
    connect(canvas_, &LightingCanvas::rigEdited, this, [this] {
        syncControls();
        rigEdited();
    });
    connect(canvas_, &LightingCanvas::lightSelected, this, &LightingEffectsDialog::selectLight);
    connect(canvas_, &LightingCanvas::lightDuplicated, this, [this](int index) {
        rebuildLightList();
        selectLight(index);
        rigEdited();
    });
    connect(canvas_, &LightingCanvas::deleteRequested, this,
            &LightingEffectsDialog::deleteSelectedLight);

    auto* column = new QVBoxLayout;
    column->setContentsMargins(0, 0, 0, 0);
    column->setSpacing(4);
    column->addWidget(buildPropertiesPanel());
    column->addWidget(buildLightsPanel(), 1);
    auto* side = new QWidget(this);
    side->setFixedWidth(kColumnWidth);
    side->setLayout(column);

    auto* body = new QHBoxLayout;
    body->setSpacing(4);
    body->addWidget(canvas_, 1);
    body->addWidget(side);

    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(4, 4, 4, 4);
    outer->setSpacing(4);
    outer->addWidget(buildOptionsBar());
    outer->addLayout(body, 1);

    rebuildLightList();
    selectLight(0);
    render();
}

QWidget* LightingEffectsDialog::buildOptionsBar()
{
    auto* bar = new QFrame(this);
    bar->setObjectName(QStringLiteral("lightingOptionsBar"));
    bar->setFrameShape(QFrame::StyledPanel);
    auto* row = new QHBoxLayout(bar);
    row->setContentsMargins(6, 3, 6, 3);
    row->setSpacing(6);

    auto* glyph = new QLabel(bar);
    glyph->setPixmap(lightGlyph(LightKind::Infinite).pixmap(20, 20));
    row->addWidget(glyph);
    row->addSpacing(6);
    row->addWidget(new QLabel(QStringLiteral("Presets:"), bar));
    presets_ = new QComboBox(bar);
    presets_->setObjectName(QStringLiteral("lightingPresets"));
    for (const LightingPreset& preset : lightingPresets()) {
        presets_->addItem(preset.name);
    }
    presets_->addItem(kCustom);
    presets_->setCurrentText(QStringLiteral("Default"));
    presets_->setMinimumContentsLength(16);
    connect(presets_, &QComboBox::textActivated, this, [this](const QString& name) {
        if (name != kCustom) {
            applyPreset(name);
        }
    });
    row->addWidget(presets_);
    row->addSpacing(12);

    row->addWidget(new QLabel(QStringLiteral("Lights:"), bar));
    const char* names[] = {"lightingAddSpot", "lightingAddPoint", "lightingAddInfinite"};
    for (int kind = 0; kind < 3; ++kind) {
        auto* add = new QToolButton(bar);
        add->setObjectName(QLatin1String(names[kind]));
        add->setIcon(lightGlyph(LightKind(kind)));
        add->setIconSize(QSize(20, 20));
        add->setAutoRaise(true);
        add->setToolTip(QStringLiteral("Add new %1 light").arg(lightKindName(LightKind(kind))));
        connect(add, &QToolButton::clicked, this, [this, kind] { addLight(LightKind(kind)); });
        addButtons_[kind] = add;
        row->addWidget(add);
    }
    auto* reset = new QPushButton(QStringLiteral("Reset"), bar);
    reset->setObjectName(QStringLiteral("lightingReset"));
    reset->setToolTip(QStringLiteral("Reset to the Default style"));
    connect(reset, &QPushButton::clicked, this, [this] { applyPreset(QStringLiteral("Default")); });
    row->addSpacing(6);
    row->addWidget(reset);
    row->addStretch(1);

    preview_ = new QCheckBox(QStringLiteral("Preview"), bar);
    preview_->setObjectName(QStringLiteral("filterPreview"));
    preview_->setChecked(true);
    connect(preview_, &QCheckBox::toggled, this, [this] { scheduleRender(); });
    row->addWidget(preview_);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), bar);
    auto* ok = new QPushButton(QStringLiteral("OK"), bar);
    ok->setDefault(true);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    row->addWidget(cancel);
    row->addWidget(ok);
    return bar;
}

LightingEffectsDialog::Field LightingEffectsDialog::makeField(
    QWidget* parent, const QString& label, const QString& name, int minimum, int maximum,
    void (LightingEffectsDialog::*apply)(int))
{
    Field field;
    field.label = new QLabel(label, parent);
    field.spin = new QSpinBox(parent);
    field.spin->setObjectName(name);
    field.spin->setRange(minimum, maximum);
    field.spin->setButtonSymbols(QAbstractSpinBox::NoButtons);
    field.spin->setAlignment(Qt::AlignRight);
    field.spin->setFixedWidth(44);
    field.label->setBuddy(field.spin);
    field.slider = new JumpSlider(Qt::Horizontal, parent);
    field.slider->setRange(minimum, maximum);
    QSpinBox* spin = field.spin;
    QSlider* slider = field.slider;
    connect(spin, &QSpinBox::valueChanged, this, [this, slider, apply](int value) {
        const QSignalBlocker block(slider);
        slider->setValue(value);
        (this->*apply)(value);
    });
    connect(slider, &QSlider::valueChanged, this, [this, spin, apply](int value) {
        const QSignalBlocker block(spin);
        spin->setValue(value);
        (this->*apply)(value);
    });
    return field;
}

ColorSwatchWidget* LightingEffectsDialog::makeSwatch(QWidget* parent, const QString& name,
                                                     const QString& title, bool rigColor)
{
    auto* swatch = new ColorSwatchWidget(parent);
    swatch->setObjectName(name);
    swatch->setFixedSize(28, 16);
    swatch->setFocusPolicy(Qt::NoFocus);
    connect(swatch, &QPushButton::clicked, this, [this, title, rigColor] {
        LightSpec* light = current();
        if (!rigColor && !light) {
            return;
        }
        const QColor before = rigColor ? rig_.colorize : light->color;
        const QColor picked = ColorPickerDialog::getColor(before, this, title);
        if (!picked.isValid() || picked == before) {
            return;
        }
        (rigColor ? rig_.colorize : light->color) = picked;
        syncControls();
        rigEdited();
    });
    return swatch;
}

QWidget* LightingEffectsDialog::buildPropertiesPanel()
{
    auto* panel = new QFrame(this);
    panel->setObjectName(QStringLiteral("lightingProperties"));
    panel->setFrameShape(QFrame::StyledPanel);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(6, 2, 6, 8);
    layout->setSpacing(3);
    layout->addWidget(panelTab(QStringLiteral("Properties"), panel));

    auto* title = new QHBoxLayout;
    auto* glyph = new QLabel(panel);
    glyph->setPixmap(lightGlyph(LightKind::Infinite).pixmap(18, 18));
    auto* name = new QLabel(QStringLiteral("Lighting Effects"), panel);
    name->setObjectName(QStringLiteral("sectionTitle"));
    title->addWidget(glyph);
    title->addWidget(name, 1);
    layout->addLayout(title);

    type_ = new QComboBox(panel);
    type_->setObjectName(QStringLiteral("lightingType"));
    for (LightKind kind : {LightKind::Spot, LightKind::Point, LightKind::Infinite}) {
        type_->addItem(lightKindName(kind));
    }
    connect(type_, &QComboBox::activated, this, [this](int index) {
        LightSpec* light = current();
        if (!light || int(light->kind) == index) {
            return;
        }
        light->kind = LightKind(index);
        rebuildLightList();
        syncControls();
        rigEdited();
    });
    layout->addWidget(type_);

    auto addRow = [&](Field& field, QPushButton* swatch, QLabel* swatchLabel) {
        auto* row = new QHBoxLayout;
        if (swatch) {
            row->addWidget(swatchLabel);
            row->addWidget(swatch);
        }
        row->addStretch(1);
        row->addWidget(field.label);
        row->addWidget(field.spin);
        layout->addSpacing(4);
        layout->addLayout(row);
        layout->addWidget(field.slider);
    };

    color_ = makeSwatch(panel, QStringLiteral("lightingColor"), QStringLiteral("Light Color"),
                        false);
    intensity_ = makeField(panel, QStringLiteral("Intensity:"), QStringLiteral("lightingIntensity"),
                           -100, 100, &LightingEffectsDialog::setIntensity);
    addRow(intensity_, color_, new QLabel(QStringLiteral("Color:"), panel));
    hotspot_ = makeField(panel, QStringLiteral("Hotspot:"), QStringLiteral("lightingHotspot"), -100,
                         100, &LightingEffectsDialog::setHotspot);
    addRow(hotspot_, nullptr, nullptr);

    auto* rule = new QFrame(panel);
    rule->setFrameShape(QFrame::HLine);
    rule->setFrameShadow(QFrame::Sunken);
    layout->addSpacing(4);
    layout->addWidget(rule);

    colorize_ = makeSwatch(panel, QStringLiteral("lightingColorize"), QStringLiteral("Colorize"),
                           true);
    exposure_ = makeField(panel, QStringLiteral("Exposure:"), QStringLiteral("lightingExposure"),
                          -100, 100, &LightingEffectsDialog::setExposure);
    addRow(exposure_, colorize_, new QLabel(QStringLiteral("Colorize:"), panel));
    gloss_ = makeField(panel, QStringLiteral("Gloss:"), QStringLiteral("lightingGloss"), -100, 100,
                       &LightingEffectsDialog::setGloss);
    addRow(gloss_, nullptr, nullptr);
    metallic_ = makeField(panel, QStringLiteral("Metallic:"), QStringLiteral("lightingMetallic"),
                          -100, 100, &LightingEffectsDialog::setMetallic);
    addRow(metallic_, nullptr, nullptr);
    ambience_ = makeField(panel, QStringLiteral("Ambience:"), QStringLiteral("lightingAmbience"),
                          -100, 100, &LightingEffectsDialog::setAmbience);
    addRow(ambience_, nullptr, nullptr);

    auto* textureRow = new QHBoxLayout;
    auto* textureLabel = new QLabel(QStringLiteral("Texture:"), panel);
    texture_ = new QComboBox(panel);
    texture_->setObjectName(QStringLiteral("lightingTexture"));
    texture_->addItems({QStringLiteral("None"), QStringLiteral("Red"), QStringLiteral("Green"),
                        QStringLiteral("Blue")});
    textureLabel->setBuddy(texture_);
    connect(texture_, &QComboBox::currentIndexChanged, this, [this](int index) {
        rig_.texture = index;
        syncControls();
        rigEdited();
    });
    textureRow->addWidget(textureLabel);
    textureRow->addWidget(texture_, 1);
    layout->addSpacing(8);
    layout->addLayout(textureRow);
    height_ = makeField(panel, QStringLiteral("Height:"), QStringLiteral("lightingHeight"), 0, 100,
                        &LightingEffectsDialog::setHeight);
    addRow(height_, nullptr, nullptr);
    return panel;
}

QWidget* LightingEffectsDialog::buildLightsPanel()
{
    auto* panel = new QFrame(this);
    panel->setObjectName(QStringLiteral("lightingLightsPanel"));
    panel->setFrameShape(QFrame::StyledPanel);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(6, 2, 6, 4);
    layout->setSpacing(3);
    layout->addWidget(panelTab(QStringLiteral("Lights"), panel));

    lights_ = new QListWidget(panel);
    lights_->setObjectName(QStringLiteral("lightingLights"));
    lights_->setSelectionMode(QAbstractItemView::SingleSelection);
    connect(lights_, &QListWidget::currentRowChanged, this, [this](int row) {
        if (row >= 0) {
            selectLight(row);
        }
    });
    layout->addWidget(lights_, 1);

    auto* footer = new QHBoxLayout;
    footer->addStretch(1);
    trash_ = new QToolButton(panel);
    trash_->setObjectName(QStringLiteral("lightingDelete"));
    trash_->setIcon(icon(QStringLiteral("layers.delete")));
    trash_->setAutoRaise(true);
    trash_->setToolTip(QStringLiteral("Delete the selected light"));
    connect(trash_, &QToolButton::clicked, this, &LightingEffectsDialog::deleteSelectedLight);
    footer->addWidget(trash_);
    layout->addLayout(footer);
    return panel;
}

void LightingEffectsDialog::setIntensity(int value)
{
    if (LightSpec* light = current()) {
        light->intensity = value;
        canvas_->update();
        rigEdited();
    }
}

void LightingEffectsDialog::setHotspot(int value)
{
    if (LightSpec* light = current()) {
        light->hotspot = value;
        canvas_->update();
        rigEdited();
    }
}

void LightingEffectsDialog::setExposure(int value)
{
    rig_.exposure = value;
    rigEdited();
}

void LightingEffectsDialog::setGloss(int value)
{
    rig_.gloss = value;
    rigEdited();
}

void LightingEffectsDialog::setMetallic(int value)
{
    rig_.metallic = value;
    rigEdited();
}

void LightingEffectsDialog::setAmbience(int value)
{
    rig_.ambience = value;
    rigEdited();
}

void LightingEffectsDialog::setHeight(int value)
{
    rig_.height = value;
    rigEdited();
}

QList<double> LightingEffectsDialog::values() const
{
    return rig_.toSlots();
}

void LightingEffectsDialog::setValues(const QList<double>& values)
{
    LightingRig rig;
    if (!LightingRig::fromSlots(values, &rig)) {
        return;
    }
    rig_ = rig;
    {
        const QSignalBlocker block(presets_);
        presets_->setCurrentText(kCustom);
    }
    rebuildLightList();
    selectLight(0);
    scheduleRender();
}

int LightingEffectsDialog::selectedLight() const
{
    return canvas_->selected();
}

void LightingEffectsDialog::selectLight(int index)
{
    if (index < 0 || index >= rig_.lights.size()) {
        return;
    }
    canvas_->setSelected(index);
    {
        const QSignalBlocker block(lights_);
        lights_->setCurrentRow(index);
    }
    syncControls();
}

bool LightingEffectsDialog::addLight(LightKind kind)
{
    if (rig_.lights.size() >= kMaxLights) {
        return false;
    }
    rig_.lights.append(defaultLight(kind));
    rebuildLightList();
    selectLight(int(rig_.lights.size()) - 1);
    rigEdited();
    return true;
}

bool LightingEffectsDialog::deleteSelectedLight()
{
    const int index = selectedLight();
    if (rig_.lights.size() <= 1 || index < 0 || index >= rig_.lights.size()) {
        return false;
    }
    rig_.lights.removeAt(index);
    rebuildLightList();
    selectLight(std::min(index, int(rig_.lights.size()) - 1));
    rigEdited();
    return true;
}

void LightingEffectsDialog::applyPreset(const QString& name)
{
    for (const LightingPreset& preset : lightingPresets()) {
        if (preset.name != name) {
            continue;
        }
        rig_ = preset.rig;
        {
            const QSignalBlocker block(presets_);
            presets_->setCurrentText(name);
        }
        rebuildLightList();
        selectLight(0);
        scheduleRender();
        return;
    }
}

QImage LightingEffectsDialog::previewImage()
{
    if (renderPending_) {
        render();
    }
    return canvas_->image();
}

LightSpec* LightingEffectsDialog::current()
{
    const int index = selectedLight();
    return index >= 0 && index < rig_.lights.size() ? &rig_.lights[index] : nullptr;
}

void LightingEffectsDialog::syncControls()
{
    LightSpec* light = current();
    if (light) {
        const QSignalBlocker block(type_);
        type_->setCurrentIndex(int(light->kind));
        paintSwatch(color_, light->color);
        intensity_.setValue(light->intensity);
        hotspot_.setValue(light->hotspot);
    }
    // CS6 greys Hotspot for the lights that have none.
    hotspot_.setEnabled(light && light->kind == LightKind::Spot);
    paintSwatch(colorize_, rig_.colorize);
    exposure_.setValue(rig_.exposure);
    gloss_.setValue(rig_.gloss);
    metallic_.setValue(rig_.metallic);
    ambience_.setValue(rig_.ambience);
    {
        const QSignalBlocker block(texture_);
        texture_->setCurrentIndex(rig_.texture);
    }
    height_.setValue(rig_.height);
    height_.setEnabled(rig_.texture != 0);
    for (QToolButton* add : addButtons_) {
        add->setEnabled(rig_.lights.size() < kMaxLights);
    }
    trash_->setEnabled(rig_.lights.size() > 1);
}

void LightingEffectsDialog::rebuildLightList()
{
    const QSignalBlocker block(lights_);
    // clear() leaves the row widgets to a deferred delete; drop them now so the
    // old rows' eyes cannot be reached.
    for (int i = 0; i < lights_->count(); ++i) {
        delete lights_->itemWidget(lights_->item(i));
    }
    lights_->clear();
    const QList<QString> names = lightNames(rig_);
    for (int i = 0; i < rig_.lights.size(); ++i) {
        auto* item = new QListWidgetItem(lights_);
        item->setSizeHint(QSize(0, 26));
        item->setData(Qt::AccessibleTextRole, names.at(i));
        auto* row = new QWidget(lights_);
        auto* layout = new QHBoxLayout(row);
        layout->setContentsMargins(2, 0, 4, 0);
        layout->setSpacing(6);
        auto* eye = new QToolButton(row);
        eye->setObjectName(QStringLiteral("lightingEye"));
        eye->setCheckable(true);
        eye->setChecked(rig_.lights.at(i).on);
        eye->setAutoRaise(true);
        eye->setIcon(icon(eye->isChecked() ? QStringLiteral("layers.eyeOn")
                                           : QStringLiteral("layers.eyeOff")));
        eye->setToolTip(QStringLiteral("Show or hide this light"));
        connect(eye, &QToolButton::toggled, this, [this, eye, i](bool on) {
            if (i < rig_.lights.size()) {
                rig_.lights[i].on = on;
                eye->setIcon(icon(on ? QStringLiteral("layers.eyeOn")
                                     : QStringLiteral("layers.eyeOff")));
                canvas_->update();
                rigEdited();
            }
        });
        auto* glyph = new QLabel(row);
        glyph->setPixmap(lightGlyph(rig_.lights.at(i).kind).pixmap(16, 16));
        auto* name = new QLabel(names.at(i), row);
        name->setObjectName(QStringLiteral("lightingName"));
        layout->addWidget(eye);
        layout->addWidget(glyph);
        layout->addWidget(name, 1);
        lights_->setItemWidget(item, row);
    }
    lights_->setCurrentRow(std::clamp(selectedLight(), 0, int(rig_.lights.size()) - 1));
}

void LightingEffectsDialog::rigEdited()
{
    {
        const QSignalBlocker block(presets_);
        presets_->setCurrentText(kCustom);
    }
    scheduleRender();
}

void LightingEffectsDialog::scheduleRender()
{
    if (renderPending_) {
        return;
    }
    renderPending_ = true;
    // Coalesce a burst of drag events into one render.
    QTimer::singleShot(0, this, [this] {
        if (renderPending_) {
            render();
        }
    });
}

void LightingEffectsDialog::render()
{
    renderPending_ = false;
    if (proxy_.isNull()) {
        canvas_->setImage(QImage());
        return;
    }
    if (!preview_->isChecked()) {
        canvas_->setImage(proxy_);
        return;
    }
    const QImage lit =
        filter_thumbnail(proxyRgba_, proxy_.width(), proxy_.height(), spec_.kind, values());
    canvas_->setImage(lit.isNull() ? proxy_ : lit);
}

bool LightingEffectsDialog::get(PictureView* view, const FilterCommandSpec& spec,
                                const QList<double>& initial, QList<double>* out,
                                QWidget* parent)
{
    LightingEffectsDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura

#include "moc_lighting_effects_dialog.cpp"
