#include "blur_gallery_dialog.h"

#include "blur_gallery_light_range.h"
#include "dialogs.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QShortcut>
#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <functional>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

namespace pictura {

namespace {

// The workspace renders a proxy this size at most; the pin positions are
// fractions of the picture, so it is the same picture OK commits.
const int kProxyEdge = 1100;
// While a pin, line, or amount slider is dragged the preview renders from a
// copy this size at most (about a tenth of the full proxy's pixels), so the
// overlay keeps up with the pointer; release renders the full proxy.
const int kDraftEdge = 360;
const int kColumnWidth = 264;
const double kMaxBlur = 500.0;
const int kEffectsTail = 13;
const int kMotionSlots = 6;
const double kMinIrisRadius = 0.0001;
const double kMaxIrisRadius = 10.0;
const double kMaxTiltExtent = 10.0;
const double kMaxSpinAngle = 360.0;
const int kSpinStrobeSlots = 3;

BlurGalleryMode modeForKind(const QString& kind)
{
    if (kind == QStringLiteral("iris-blur")) {
        return BlurGalleryMode::Iris;
    }
    if (kind == QStringLiteral("path-blur")) {
        return BlurGalleryMode::Path;
    }
    if (kind == QStringLiteral("spin-blur")) {
        return BlurGalleryMode::Spin;
    }
    return kind == QStringLiteral("tilt-shift") ? BlurGalleryMode::TiltShift
                                                : BlurGalleryMode::Field;
}

} // namespace

void BlurGalleryDialog::Field::setValue(double value)
{
    const QSignalBlocker blockSpin(spin);
    const QSignalBlocker blockSlider(slider);
    spin->setValue(value);
    slider->setValue(qRound(value));
}

void BlurGalleryDialog::Field::setEnabled(bool on)
{
    label->setEnabled(on);
    spin->setEnabled(on);
    slider->setEnabled(on);
}

BlurGalleryDialog::BlurGalleryDialog(PictureView* view, const FilterCommandSpec& spec,
                                     QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec), active_(modeForKind(spec.kind))
{
    setWindowTitle(spec.label);
    setObjectName(QStringLiteral("blurGalleryDialog"));
    const QWidget* frame = parent ? parent->window() : nullptr;
    resize(frame ? (QSizeF(frame->size()) * 0.92).toSize() : QSize(1100, 720));

    canvas_ = new BlurGalleryCanvas(nullptr, this);
    if (view_ && view_->has_document()) {
        const QImage image = documentPreviewImage(view_, kProxyEdge);
        if (!image.isNull()) {
            proxy_ = (image.width() > kProxyEdge || image.height() > kProxyEdge
                          ? image.scaled(kProxyEdge, kProxyEdge, Qt::KeepAspectRatio,
                                         Qt::SmoothTransformation)
                          : image)
                         .convertToFormat(QImage::Format_ARGB32_Premultiplied);
            const int longEdge = qMax(view_->document_width(), view_->document_height());
            proxyEdge_ = qMin(kProxyEdge, longEdge);
            draftEdge_ = proxyEdge_ > kDraftEdge ? kDraftEdge : 0;
            const auto base = [this](int edge) {
                return edge > 0 ? filter_proxy_base(*view_, edge).convertToFormat(
                                      QImage::Format_ARGB32_Premultiplied)
                                : QImage();
            };
            base_ = base(proxyEdge_);
            draftBase_ = base(draftEdge_);
            canvas_->setDocumentSize(QSize(view_->document_width(), view_->document_height()));
        }
    }
    connect(canvas_, &BlurGalleryCanvas::pinsEdited, this, &BlurGalleryDialog::pinsEdited);
    connect(canvas_, &BlurGalleryCanvas::pinSelected, this, &BlurGalleryDialog::selectPin);
    connect(canvas_, &BlurGalleryCanvas::pinAdded, this, [this](int index) { selectPin(index); });
    connect(canvas_, &BlurGalleryCanvas::dragFinished, this, &BlurGalleryDialog::scheduleRender);
    connect(canvas_, &BlurGalleryCanvas::deleteRequested, this,
            &BlurGalleryDialog::deleteSelectedPin);

    auto* column = new QVBoxLayout;
    column->setContentsMargins(0, 0, 0, 0);
    column->setSpacing(4);
    column->addWidget(buildBlurToolsPanel());
    column->addStretch(1);
    column->addWidget(buildEffectsPanel());
    column->addStretch(1);
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
    // The window can never be squeezed below what the side column needs, so
    // every slider and value box keeps its height; the floor grows as rows
    // expand. An explicit minimum size would override this.
    outer->setSizeConstraint(QLayout::SetMinimumSize);
    outer->addLayout(body, 1);
    outer->addWidget(buildOptionsBar());

    auto* hide = new QShortcut(QKeySequence(Qt::Key_H), this);
    hide->setContext(Qt::WidgetWithChildrenShortcut);
    connect(hide, &QShortcut::activated, this,
            [this] { canvas_->setUiVisible(!canvas_->uiVisible()); });

    selectEffect(active_);
    effectsEdited();
    render();
}

QWidget* BlurGalleryDialog::buildOptionsBar()
{
    auto* bar = new QFrame(this);
    bar->setObjectName(QStringLiteral("blurGalleryOptionsBar"));
    bar->setFrameShape(QFrame::StyledPanel);
    auto* row = new QHBoxLayout(bar);
    row->setContentsMargins(6, 3, 6, 3);
    row->setSpacing(6);

    auto* title = new QLabel(QStringLiteral("Blur Gallery"), bar);
    title->setObjectName(QStringLiteral("dialogTitle"));
    row->addWidget(title);
    row->addSpacing(12);
    hint_ = new QLabel(bar);
    hint_->setEnabled(false);
    // A long hint clips rather than setting the workspace's minimum width.
    hint_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    row->addWidget(hint_);
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

QWidget* BlurGalleryDialog::effectRow(QWidget* parent, const QString& label,
                                      const QString& name, Effect* effect)
{
    auto* row = new QWidget(parent);
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);
    auto* triangle = new QToolButton(row);
    triangle->setObjectName(name + QStringLiteral("EnabledArrow"));
    triangle->setAutoRaise(true);
    triangle->setText(QStringLiteral("▶"));
    // The name selects the effect, as clicking a Blur Tools row does in CS6.
    auto* title = new QToolButton(row);
    title->setObjectName(name + QStringLiteral("Select"));
    title->setAutoRaise(true);
    title->setText(label);
    title->setToolButtonStyle(Qt::ToolButtonTextOnly);
    auto* check = new QCheckBox(row);
    check->setObjectName(name + QStringLiteral("Enabled"));
    layout->addWidget(triangle);
    layout->addWidget(title);
    layout->addStretch(1);
    layout->addWidget(check);
    if (!effect) {
        for (QWidget* widget : {static_cast<QWidget*>(triangle), static_cast<QWidget*>(title),
                                static_cast<QWidget*>(check)}) {
            widget->setEnabled(false);
        }
        row->setToolTip(QStringLiteral("Not implemented yet"));
        return row;
    }
    effect->check = check;
    effect->arrow = triangle;
    effect->title = title;
    return row;
}

QDoubleSpinBox* BlurGalleryDialog::makeValueBox(QWidget* parent, const QString& name,
                                                double minimum, double maximum, double value)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(name);
    box->setRange(minimum, maximum);
    box->setDecimals(0);
    box->setButtonSymbols(QAbstractSpinBox::NoButtons);
    box->setAlignment(Qt::AlignRight);
    box->setFixedWidth(74);
    box->setValue(value);
    return box;
}

BlurGalleryDialog::Field BlurGalleryDialog::addField(QVBoxLayout* layout, QWidget* parent,
                                                     const QString& label, const QString& name,
                                                     double minimum, double maximum, double value,
                                                     int decimals, const QString& suffix)
{
    Field field;
    auto* row = new QHBoxLayout;
    field.label = new QLabel(label, parent);
    field.spin = makeValueBox(parent, name + QStringLiteral("Spin"), minimum, maximum, value);
    field.spin->setDecimals(decimals);
    if (!suffix.isEmpty()) {
        field.spin->setSuffix(suffix);
    }
    field.label->setBuddy(field.spin);
    field.slider = new JumpSlider(Qt::Horizontal, parent);
    field.slider->setObjectName(name + QStringLiteral("Slider"));
    field.slider->setRange(int(minimum), int(maximum));
    field.slider->setValue(qRound(value));
    row->addWidget(field.label);
    row->addStretch(1);
    row->addWidget(field.spin);
    layout->addLayout(row);
    layout->addWidget(field.slider);

    QDoubleSpinBox* spin = field.spin;
    QSlider* slider = field.slider;
    connect(spin, &QDoubleSpinBox::valueChanged, this, [this, slider](double v) {
        const QSignalBlocker block(slider);
        slider->setValue(qRound(v));
        scheduleRender();
    });
    connect(slider, &QSlider::valueChanged, this, [this, spin](int v) {
        const QSignalBlocker block(spin);
        spin->setValue(v);
        scheduleRender();
    });
    return field;
}

QWidget* BlurGalleryDialog::buildBlurToolsPanel()
{
    auto* panel = new QFrame(this);
    panel->setObjectName(QStringLiteral("blurToolsPanel"));
    panel->setFrameShape(QFrame::StyledPanel);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(6, 2, 6, 8);
    layout->setSpacing(3);

    auto* tab = new QLabel(QStringLiteral("Blur Tools"), panel);
    tab->setObjectName(QStringLiteral("sectionTabLabel"));
    layout->addWidget(tab);

    const struct {
        const char* label;
        const char* name;
        int mode;
    } rows[] = {
        {"Field Blur", "fieldBlur", int(BlurGalleryMode::Field)},
        {"Iris Blur", "irisBlur", int(BlurGalleryMode::Iris)},
        {"Tilt-Shift", "tiltShift", int(BlurGalleryMode::TiltShift)},
        {"Path Blur", "pathBlur", int(BlurGalleryMode::Path)},
        {"Spin Blur", "spinBlur", int(BlurGalleryMode::Spin)},
    };
    for (const auto& row : rows) {
        const QString name = QLatin1String(row.name);
        Effect* effect = row.mode >= 0 ? &effects_[row.mode] : nullptr;
        layout->addWidget(effectRow(panel, QLatin1String(row.label), name, effect));
        if (!effect) {
            continue;
        }
        const auto mode = BlurGalleryMode(row.mode);
        if (mode == BlurGalleryMode::Path || mode == BlurGalleryMode::Spin) {
            effect->title->setToolTip(
                QStringLiteral("A post-CS6 (Photoshop CC) effect; the blur is approximated"));
        }
        effect->controls = buildEffectControls(panel, mode, name);
        effect->controls->setVisible(false);
        layout->addWidget(effect->controls);
        connect(effect->arrow, &QToolButton::clicked, this,
                [this, mode] { setExpanded(mode, effects_[int(mode)].controls->isHidden()); });
        connect(effect->title, &QToolButton::clicked, this, [this, mode] { selectEffect(mode); });
        connect(effect->check, &QCheckBox::toggled, this,
                [this, mode](bool on) { setEffectEnabled(mode, on); });
    }
    return panel;
}

QWidget* BlurGalleryDialog::buildEffectControls(QWidget* parent, BlurGalleryMode mode,
                                                const QString& name)
{
    Effect& effect = effects_[int(mode)];
    auto* box = new QWidget(parent);
    auto* controls = new QVBoxLayout(box);
    controls->setContentsMargins(18, 0, 0, 0);
    controls->setSpacing(3);
    if (mode == BlurGalleryMode::Path) {
        buildPathControls(box, controls);
        return box;
    }
    // Spin Blur's amount is its Blur Angle.
    const bool spin = mode == BlurGalleryMode::Spin;
    const double top = spin ? kMaxSpinAngle : kMaxBlur;
    auto* blurRow = new QHBoxLayout;
    auto* label = new QLabel(spin ? QStringLiteral("Blur Angle:") : QStringLiteral("Blur:"), box);
    effect.blurSpin = makeValueBox(box, name + QStringLiteral("Spin"), 0.0, top, 0.0);
    effect.blurSpin->setSuffix(spin ? QStringLiteral("°") : QStringLiteral(" px"));
    label->setBuddy(effect.blurSpin);
    blurRow->addWidget(label);
    blurRow->addStretch(1);
    blurRow->addWidget(effect.blurSpin);
    controls->addLayout(blurRow);
    effect.blurSlider = new JumpSlider(Qt::Horizontal, box);
    effect.blurSlider->setObjectName(name + QStringLiteral("Slider"));
    effect.blurSlider->setRange(0, int(top));
    controls->addWidget(effect.blurSlider);
    // An amount edits the effect's own selected pin, selecting the effect.
    const auto setBlur = [this, mode](double value) {
        selectEffect(mode);
        setSelectedBlur(value);
    };
    connect(effect.blurSpin, &QDoubleSpinBox::valueChanged, this, setBlur);
    connect(effect.blurSlider, &QSlider::valueChanged, this, setBlur);
    connect(effect.blurSlider, &QSlider::sliderReleased, this, &BlurGalleryDialog::scheduleRender);
    if (mode != BlurGalleryMode::TiltShift) {
        return box;
    }
    auto* distortionRow = new QHBoxLayout;
    auto* distortionLabel = new QLabel(QStringLiteral("Distortion:"), box);
    distortionSpin_ = makeValueBox(box, QStringLiteral("tiltDistortionSpin"), -100, 100, 0);
    distortionSpin_->setSuffix(QStringLiteral("%"));
    distortionLabel->setBuddy(distortionSpin_);
    distortionRow->addWidget(distortionLabel);
    distortionRow->addStretch(1);
    distortionRow->addWidget(distortionSpin_);
    controls->addLayout(distortionRow);
    distortionSlider_ = new JumpSlider(Qt::Horizontal, box);
    distortionSlider_->setObjectName(QStringLiteral("tiltDistortionSlider"));
    distortionSlider_->setRange(-100, 100);
    controls->addWidget(distortionSlider_);
    symmetric_ = new QCheckBox(QStringLiteral("Symmetric Distortion"), box);
    symmetric_->setObjectName(QStringLiteral("tiltSymmetric"));
    controls->addWidget(symmetric_);
    const auto setDistortion = [this](double value, bool symmetric) {
        selectEffect(BlurGalleryMode::TiltShift);
        setSelectedDistortion(value, symmetric);
    };
    connect(distortionSpin_, &QDoubleSpinBox::valueChanged, this,
            [this, setDistortion](double v) { setDistortion(v, symmetric_->isChecked()); });
    connect(distortionSlider_, &QSlider::valueChanged, this,
            [this, setDistortion](int v) { setDistortion(v, symmetric_->isChecked()); });
    connect(distortionSlider_, &QSlider::sliderReleased, this,
            &BlurGalleryDialog::scheduleRender);
    connect(symmetric_, &QCheckBox::toggled, this,
            [this, setDistortion](bool on) { setDistortion(distortionSpin_->value(), on); });
    return box;
}

QWidget* BlurGalleryDialog::buildEffectsPanel()
{
    auto* panel = new QFrame(this);
    panel->setObjectName(QStringLiteral("blurEffectsPanel"));
    panel->setFrameShape(QFrame::StyledPanel);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(2, 2, 2, 2);
    effectsTabs_ = new QTabWidget(panel);
    effectsTabs_->setObjectName(QStringLiteral("blurEffectsTabs"));
    effectsTabs_->addTab(buildBokehTab(), QStringLiteral("Effects"));
    effectsTabs_->addTab(buildMotionTab(), QStringLiteral("Motion Effects"));
    effectsTabs_->addTab(buildNoiseTab(), QStringLiteral("Noise"));
    layout->addWidget(effectsTabs_);
    return panel;
}

QWidget* BlurGalleryDialog::buildBokehTab()
{
    auto* tab = new QWidget(this);
    auto* layout = new QVBoxLayout(tab);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(3);

    auto* row = new QHBoxLayout;
    auto* name = new QLabel(QStringLiteral("Bokeh"), tab);
    name->setObjectName(QStringLiteral("sectionTitle"));
    bokehEnabled_ = new QCheckBox(tab);
    bokehEnabled_->setObjectName(QStringLiteral("bokehEnabled"));
    bokehEnabled_->setChecked(true);
    row->addWidget(name);
    row->addStretch(1);
    row->addWidget(bokehEnabled_);
    layout->addLayout(row);

    bokehLight_ = addField(layout, tab, QStringLiteral("Light Bokeh:"),
                           QStringLiteral("bokehLight"), 0, 100, 14, 0, QStringLiteral(" %"));
    bokehColor_ = addField(layout, tab, QStringLiteral("Bokeh Color:"),
                           QStringLiteral("bokehColor"), 0, 100, 40, 0, QStringLiteral(" %"));

    layout->addSpacing(4);
    layout->addWidget(new QLabel(QStringLiteral("Light Range:"), tab));
    bokehRange_ = new LightRangeSlider(tab);
    layout->addWidget(bokehRange_);
    auto* spins = new QHBoxLayout;
    bokehRangeLow_ = makeValueBox(tab, QStringLiteral("bokehRangeLow"), 0, 255, 40);
    bokehRangeHigh_ = makeValueBox(tab, QStringLiteral("bokehRangeHigh"), 0, 255, 118);
    spins->addWidget(bokehRangeLow_);
    spins->addStretch(1);
    spins->addWidget(bokehRangeHigh_);
    layout->addLayout(spins);

    connect(bokehEnabled_, &QCheckBox::toggled, this, [this] { effectsEdited(); });
    bokehRange_->changed = [this](int low, int high) {
        const QSignalBlocker blockLow(bokehRangeLow_);
        const QSignalBlocker blockHigh(bokehRangeHigh_);
        bokehRangeLow_->setValue(low);
        bokehRangeHigh_->setValue(high);
        scheduleRender();
    };
    connect(bokehRangeLow_, &QDoubleSpinBox::valueChanged, this, [this](double) {
        bokehRange_->setRange(int(bokehRangeLow_->value()), int(bokehRangeHigh_->value()));
        scheduleRender();
    });
    connect(bokehRangeHigh_, &QDoubleSpinBox::valueChanged, this, [this](double) {
        bokehRange_->setRange(int(bokehRangeLow_->value()), int(bokehRangeHigh_->value()));
        scheduleRender();
    });
    return tab;
}

QWidget* BlurGalleryDialog::buildNoiseTab()
{
    auto* tab = new QWidget(this);
    auto* layout = new QVBoxLayout(tab);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(3);

    auto* row = new QHBoxLayout;
    noiseKind_ = new QComboBox(tab);
    noiseKind_->setObjectName(QStringLiteral("noiseKind"));
    noiseKind_->addItems(
        {QStringLiteral("Gaussian"), QStringLiteral("Uniform"), QStringLiteral("Grain")});
    noiseEnabled_ = new QCheckBox(tab);
    noiseEnabled_->setObjectName(QStringLiteral("noiseEnabled"));
    // On by default as in the owner's CC screenshots; Amount 0 adds nothing.
    noiseEnabled_->setChecked(true);
    row->addWidget(noiseKind_, 1);
    row->addWidget(noiseEnabled_);
    layout->addLayout(row);

    noiseAmount_ = addField(layout, tab, QStringLiteral("Amount:"), QStringLiteral("noiseAmount"),
                            0, 100, 0, 2, QStringLiteral(" %"));
    noiseSize_ = addField(layout, tab, QStringLiteral("Size:"), QStringLiteral("noiseSize"), 0, 100,
                          50, 0, QStringLiteral(" %"));
    noiseRoughness_ = addField(layout, tab, QStringLiteral("Roughness:"),
                               QStringLiteral("noiseRoughness"), 0, 100, 50, 0,
                               QStringLiteral(" %"));
    noiseColor_ = addField(layout, tab, QStringLiteral("Color:"), QStringLiteral("noiseColor"), 0,
                           100, 0, 0, QStringLiteral(" %"));
    noiseHighlights_ = addField(layout, tab, QStringLiteral("Highlights:"),
                                QStringLiteral("noiseHighlights"), 0, 100, 0, 0,
                                QStringLiteral(" %"));

    connect(noiseEnabled_, &QCheckBox::toggled, this, [this] { effectsEdited(); });
    connect(noiseKind_, &QComboBox::currentIndexChanged, this, [this] { scheduleRender(); });
    return tab;
}

namespace {

const char* const kEffectKinds[] = {"field-blur", "iris-blur", "tilt-shift", "path-blur",
                                    "spin-blur"};
// Path Blur's records carry their own point count (`readPaths`); Spin
// Blur's are read by `readSpins`.
const int kPinStrides[] = {3, 8, 10};

} // namespace

QString BlurGalleryDialog::kind() const
{
    int on = 0;
    int last = 0;
    for (int m = 0; m < kEffects; ++m) {
        if (effects_[m].enabled && !effects_[m].pins.isEmpty()) {
            ++on;
            last = m;
        }
    }
    if (on == 0) {
        return QString();
    }
    return on == 1 ? QLatin1String(kEffectKinds[last]) : QStringLiteral("blur-gallery");
}

QList<double> BlurGalleryDialog::values() const
{
    return slotList(1.0);
}

void BlurGalleryDialog::appendPins(QList<double>* out, BlurGalleryMode mode, double scale) const
{
    if (mode == BlurGalleryMode::Spin) {
        appendSpins(out);
        return;
    }
    const Effect& effect = effects_[int(mode)];
    const bool on = effect.enabled;
    *out << (on ? effect.pins.size() : 0);
    if (!on) {
        return;
    }
    for (const BlurGalleryPin& pin : effect.pins) {
        *out << pin.pos.x() << pin.pos.y();
        if (mode == BlurGalleryMode::Iris) {
            *out << pin.radii.width() << pin.radii.height() << pin.angle << pin.squareness
                 << pin.feather;
        } else if (mode == BlurGalleryMode::TiltShift) {
            *out << pin.angle << pin.focus[0] << pin.focus[1] << pin.fade[0] << pin.fade[1]
                 << pin.distortion << (pin.symmetric ? 1.0 : 0.0);
        }
        *out << pin.blur * scale;
    }
}

QList<double> BlurGalleryDialog::slotList(double scale) const
{
    QList<double> out;
    const QString commit = kind();
    if (commit.isEmpty()) {
        return out;
    }
    const bool combined = commit == QStringLiteral("blur-gallery");
    for (int m = 0; m < kEffects; ++m) {
        if (!combined && commit != QLatin1String(kEffectKinds[m])) {
            continue;
        }
        if (BlurGalleryMode(m) == BlurGalleryMode::Path) {
            appendPaths(&out, scale);
        } else {
            appendPins(&out, BlurGalleryMode(m), scale);
        }
    }
    // The motion section follows the last effect's records; the combined
    // list adds the flash duration, Spin Blur alone takes its three strobe
    // slots.
    if (combined || commit == QStringLiteral("path-blur")) {
        appendMotion(&out);
    }
    if (combined) {
        out << strobeDuration_.spin->value();
    } else if (commit == QStringLiteral("spin-blur")) {
        appendSpinStrobe(&out);
    }
    out << (bokehEnabled_->isChecked() ? 1.0 : 0.0) << bokehLight_.spin->value()
        << bokehColor_.spin->value() << bokehRangeLow_->value() << bokehRangeHigh_->value()
        << (noiseEnabled_->isChecked() ? 1.0 : 0.0) << noiseAmount_.spin->value()
        << noiseSize_.spin->value() << noiseRoughness_.spin->value() << noiseColor_.spin->value()
        << noiseHighlights_.spin->value() << double(noiseKind_->currentIndex()) << noiseSeed_;
    return out;
}

bool BlurGalleryDialog::readPins(const QList<double>& values, int* at, BlurGalleryMode mode,
                                 QList<BlurGalleryPin>* pins) const
{
    if (mode == BlurGalleryMode::Path) {
        return readPaths(values, at, pins);
    }
    if (mode == BlurGalleryMode::Spin) {
        return readSpins(values, at, pins);
    }
    const int stride = kPinStrides[int(mode)];
    if (*at >= values.size()) {
        return false;
    }
    const double count = values.at(*at);
    if (count < 0 || count > 16 || count != double(int(count))
        || *at + 1 + stride * int(count) > values.size()) {
        return false;
    }
    for (int i = 0; i < int(count); ++i) {
        const auto slot = [&](int k) { return values.at(*at + 1 + stride * i + k); };
        BlurGalleryPin pin;
        pin.pos = QPointF(slot(0), slot(1));
        if (mode == BlurGalleryMode::Iris) {
            pin.radii = QSizeF(qBound(kMinIrisRadius, slot(2), kMaxIrisRadius),
                               qBound(kMinIrisRadius, slot(3), kMaxIrisRadius));
            pin.angle = slot(4);
            pin.squareness = qBound(0.0, slot(5), 100.0);
            pin.feather = qBound(0.0, slot(6), 1.0);
        } else if (mode == BlurGalleryMode::TiltShift) {
            pin.angle = slot(2);
            for (int k = 0; k < 2; ++k) {
                pin.focus[k] = qBound(0.0, slot(3 + k), kMaxTiltExtent);
                pin.fade[k] = qBound(0.0, slot(5 + k), kMaxTiltExtent);
            }
            pin.distortion = qBound(-100.0, slot(7), 100.0);
            pin.symmetric = slot(8) >= 0.5;
        }
        pin.blur = qBound(0.0, slot(stride - 1), kMaxBlur);
        pins->append(pin);
    }
    *at += 1 + stride * int(count);
    return true;
}

void BlurGalleryDialog::readTail(const QList<double>& values, int tail)
{
    {
        const QSignalBlocker blockBokeh(bokehEnabled_);
        const QSignalBlocker blockNoise(noiseEnabled_);
        const QSignalBlocker blockKind(noiseKind_);
        bokehEnabled_->setChecked(values.at(tail) >= 0.5);
        bokehLight_.setValue(values.at(tail + 1));
        bokehColor_.setValue(values.at(tail + 2));
        bokehRangeLow_->setValue(values.at(tail + 3));
        bokehRangeHigh_->setValue(values.at(tail + 4));
        noiseEnabled_->setChecked(values.at(tail + 5) >= 0.5);
        noiseAmount_.setValue(values.at(tail + 6));
        noiseSize_.setValue(values.at(tail + 7));
        noiseRoughness_.setValue(values.at(tail + 8));
        noiseColor_.setValue(values.at(tail + 9));
        noiseHighlights_.setValue(values.at(tail + 10));
        noiseKind_->setCurrentIndex(qBound(0, int(values.at(tail + 11)), 2));
        noiseSeed_ = values.at(tail + 12);
    }
    bokehRange_->setRange(int(bokehRangeLow_->value()), int(bokehRangeHigh_->value()));
}

void BlurGalleryDialog::setValues(const QList<double>& values)
{
    // The opening command's layout: one effect's list, or all five in turn,
    // then the motion section (and the flash duration), Path Blur's records
    // followed by the motion section, or Spin Blur's by its strobe slots.
    const bool combined = spec_.kind == QStringLiteral("blur-gallery");
    const BlurGalleryMode opening = modeForKind(spec_.kind);
    const bool motion = combined || opening == BlurGalleryMode::Path;
    const bool spinStrobe = !combined && opening == BlurGalleryMode::Spin;
    QList<BlurGalleryPin> parsed[kEffects];
    int at = 0;
    bool whole = !values.isEmpty();
    for (int m = 0; m < kEffects && whole; ++m) {
        if (combined || BlurGalleryMode(m) == opening) {
            whole = readPins(values, &at, BlurGalleryMode(m), &parsed[m]);
        }
    }
    const int motionAt = at;
    if (motion) {
        at += kMotionSlots + (combined ? 1 : 0);
    } else if (spinStrobe) {
        at += kSpinStrobeSlots;
    }
    whole = whole && values.size() == at + kEffectsTail
            && (combined || !parsed[int(opening)].isEmpty());
    canvas_->setPins(nullptr, 0);
    for (int m = 0; m < kEffects; ++m) {
        Effect& effect = effects_[m];
        effect.pins = whole ? parsed[m] : QList<BlurGalleryPin>();
        effect.selected = 0;
        effect.enabled = !effect.pins.isEmpty();
        const QSignalBlocker block(effect.check);
        effect.check->setChecked(effect.enabled);
        setExpanded(BlurGalleryMode(m), false);
    }
    if (whole) {
        if (motion) {
            readMotion(values, motionAt);
        }
        if (combined) {
            setStrobeDuration(values.at(motionAt + kMotionSlots));
        } else if (spinStrobe) {
            readSpinStrobe(values, motionAt);
        }
        readTail(values, at);
    }
    BlurGalleryMode first = opening;
    for (int m = kEffects - 1; m >= 0; --m) {
        if (effects_[m].enabled && (combined || BlurGalleryMode(m) == opening)) {
            first = BlurGalleryMode(m);
        }
    }
    selectEffect(first);
    effectsEdited();
}

BlurGalleryPin BlurGalleryDialog::defaultPin(BlurGalleryMode mode) const
{
    BlurGalleryPin pin;
    if (mode == BlurGalleryMode::Path) {
        // The owner's opening screenshot (at 100 %): a short stroke right
        // from the centre with blur shapes about half its length.
        const double width =
            view_ && view_->has_document() ? double(view_->document_width()) : 830.0;
        pin.points = {QPointF(0.5, 0.5), QPointF(0.58, 0.5)};
        pin.speeds[0] = pin.speeds[1] = qRound(0.04 * width);
        return pin;
    }
    if (mode == BlurGalleryMode::Spin) {
        // The owner's opening screenshot: a centred circle 0.13 of the width
        // across at 15°, as `pictura_filters::SpinPin::default()`.
        pin.radii = QSizeF(0.13, 0.13);
        pin.feather = 0.5;
        pin.blur = 15.0;
        return pin;
    }
    if (mode == BlurGalleryMode::TiltShift) {
        // The band's extents open at `pictura_filters::TiltPin::default()`.
        pin.blur = 15.0;
        return pin;
    }
    if (mode != BlurGalleryMode::Iris) {
        return pin;
    }
    // The owner's screenshot opens on an outline proportioned like the
    // picture; 15 px is CS6's Iris default (unverified).
    pin.blur = 15.0;
    if (view_ && view_->has_document() && view_->document_width() > 0
        && view_->document_height() > 0) {
        pin.radii.setHeight(pin.radii.width() * double(view_->document_height())
                            / double(view_->document_width()));
    }
    return pin;
}

void BlurGalleryDialog::selectEffect(BlurGalleryMode mode)
{
    // Remember the pin the canvas had selected on the effect it was showing.
    if (canvas_->mode() == active_) {
        effects_[int(active_)].selected = canvas_->selected();
    }
    active_ = mode;
    Effect& effect = effects_[int(mode)];
    if (effect.pins.isEmpty()) {
        effect.pins.append(defaultPin(mode));
        effect.selected = 0;
    }
    if (!effect.enabled) {
        effect.enabled = true;
        const QSignalBlocker block(effect.check);
        effect.check->setChecked(true);
    }
    // The edited effect's name reads bold.
    for (int m = 0; m < kEffects; ++m) {
        QFont font = effects_[m].title->font();
        font.setBold(m == int(mode));
        effects_[m].title->setFont(font);
    }
    canvas_->setMode(mode);
    canvas_->setPins(&effect.pins, effect.selected);
    setExpanded(mode, true);
    syncMotionTab();
    if (hint_) {
        const QString hints[] = {
            QStringLiteral("Click the image to add a pin, drag a pin to move it, its ring to "
                           "blur, H to hide the pins"),
            QStringLiteral("Click to add an iris; drag its handles to size, turn, feather, or "
                           "square it; H hides the overlay"),
            QStringLiteral("Click to add a band; drag its lines to set the sharp and fade "
                           "areas, a dot to turn it; H hides the overlay"),
            QStringLiteral("Click to pin stop points, double-click to end; drag a point to move "
                           "it, Ctrl-drag a path; H hides the paths (post-CS6)"),
            QStringLiteral("Click to add a spin; drag its handles to size or turn it, its ring "
                           "to spin, Alt-drag its centre to move the rotation point (post-CS6)"),
        };
        hint_->setText(hints[int(mode)]);
    }
    syncControls();
    scheduleRender();
}

void BlurGalleryDialog::setEffectEnabled(BlurGalleryMode mode, bool on)
{
    if (on) {
        selectEffect(mode);
        return;
    }
    effects_[int(mode)].enabled = false;
    scheduleRender();
}

void BlurGalleryDialog::setExpanded(BlurGalleryMode mode, bool on)
{
    // One row open at a time: opening a row closes the one that was open, so
    // the column (and the workspace's minimum height) stays short.
    for (int m = 0; m < kEffects; ++m) {
        if (m != int(mode) && !on) {
            continue;
        }
        const bool open = on && m == int(mode);
        effects_[m].controls->setVisible(open);
        effects_[m].arrow->setText(open ? QStringLiteral("▼") : QStringLiteral("▶"));
    }
}

void BlurGalleryDialog::selectPin(int index)
{
    if (index < 0 || index >= pins().size()) {
        return;
    }
    canvas_->setSelected(index);
    syncControls();
}

void BlurGalleryDialog::pinsEdited()
{
    syncControls();
    scheduleRender();
}

void BlurGalleryDialog::deleteSelectedPin()
{
    if (active_ == BlurGalleryMode::Path) {
        deleteSelectedPathPart();
        return;
    }
    const int index = canvas_->selected();
    if (pins().size() <= 1 || index < 0 || index >= pins().size()) {
        return;
    }
    pins().removeAt(index);
    canvas_->setSelected(qMin(index, int(pins().size()) - 1));
    syncControls();
    canvas_->update();
    scheduleRender();
}

void BlurGalleryDialog::setSelectedBlur(double blur)
{
    const int index = canvas_->selected();
    if (index < 0 || index >= pins().size()) {
        return;
    }
    pins()[index].blur =
        qBound(0.0, blur, active_ == BlurGalleryMode::Spin ? kMaxSpinAngle : kMaxBlur);
    syncControls();
    canvas_->update();
    scheduleRender();
}

void BlurGalleryDialog::setSelectedDistortion(double distortion, bool symmetric)
{
    const int index = canvas_->selected();
    if (index < 0 || index >= pins().size()) {
        return;
    }
    pins()[index].distortion = qBound(-100.0, distortion, 100.0);
    pins()[index].symmetric = symmetric;
    syncControls();
    scheduleRender();
}

void BlurGalleryDialog::syncControls()
{
    // Each row shows its own effect's selected pin.
    syncPathControls();
    for (int m = 0; m < kEffects; ++m) {
        Effect& effect = effects_[m];
        const int index = m == int(active_) ? canvas_->selected() : effect.selected;
        if (!effect.blurSpin || index < 0 || index >= effect.pins.size()) {
            continue;
        }
        const BlurGalleryPin& pin = effect.pins.at(index);
        const QSignalBlocker blockSpin(effect.blurSpin);
        const QSignalBlocker blockSlider(effect.blurSlider);
        effect.blurSpin->setValue(pin.blur);
        effect.blurSlider->setValue(qRound(pin.blur));
        if (BlurGalleryMode(m) == BlurGalleryMode::TiltShift) {
            const QSignalBlocker blockDistortion(distortionSpin_);
            const QSignalBlocker blockDistortionSlider(distortionSlider_);
            const QSignalBlocker blockSymmetric(symmetric_);
            distortionSpin_->setValue(pin.distortion);
            distortionSlider_->setValue(qRound(pin.distortion));
            symmetric_->setChecked(pin.symmetric);
        }
    }
}

void BlurGalleryDialog::effectsEdited()
{
    // The tab controls stay editable whatever their checkbox says: the
    // checkbox only gates the effect at render and commit time, so values can
    // be dialled in before it is switched on.
    scheduleRender();
}

void BlurGalleryDialog::scheduleRender()
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

bool BlurGalleryDialog::interacting() const
{
    const auto held = [](const QSlider* slider) { return slider && slider->isSliderDown(); };
    bool any = canvas_->dragging() || held(distortionSlider_) || held(pathSpeed_.slider)
               || held(pathTaper_.slider) || held(pathEndSpeed_.slider)
               || held(strobeStrength_.slider) || held(strobeFlashes_.slider)
               || held(strobeDuration_.slider);
    for (const Effect& effect : effects_) {
        any = any || held(effect.blurSlider);
    }
    return any;
}

void BlurGalleryDialog::render()
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
    const bool draft = !draftBase_.isNull() && interacting();
    const QImage& base = draft ? draftBase_ : base_;
    const int edge = draft ? draftEdge_ : proxyEdge_;
    if (base.isNull()) {
        canvas_->setImage(proxy_);
        return;
    }
    // Pin amounts are document pixels; the layer is filtered at proxy scale.
    const double scale =
        double(edge) / double(qMax(view_->document_width(), view_->document_height()));
    const QString commit = kind();
    if (commit.isEmpty()) {
        canvas_->setImage(proxy_);
        return;
    }
    const QImage layer = filter_proxy_layer(*view_, commit, slotList(scale), edge);
    QImage composed = base.copy();
    if (!layer.isNull()) {
        QPainter painter(&composed);
        painter.drawImage(0, 0, layer);
    }
    canvas_->setImage(composed);
}

bool BlurGalleryDialog::get(PictureView* view, const FilterCommandSpec& spec,
                            const QList<double>& initial, QList<double>* out, QString* kind,
                            QWidget* parent)
{
    BlurGalleryDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    // OK while a path is being placed keeps the stops pinned so far.
    dialog.canvas()->finishPath();
    if (dialog.kind().isEmpty()) {
        return false;
    }
    *kind = dialog.kind();
    *out = dialog.values();
    return true;
}

} // namespace pictura
