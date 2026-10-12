#include "blur_gallery_dialog.h"

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
const int kColumnWidth = 264;
const double kMaxBlur = 500.0;
const int kEffectsTail = 13;
const double kMinIrisRadius = 0.0001;
const double kMaxIrisRadius = 10.0;

} // namespace

// The Light Range control: a black-to-white bar with a low and a high handle,
// 0..255. The dialog drives it and reads it back; `changed` fires on a drag.
class LightRangeSlider : public QWidget {
public:
    explicit LightRangeSlider(QWidget* parent = nullptr) : QWidget(parent)
    {
        setObjectName(QStringLiteral("bokehRange"));
        setMinimumHeight(20);
        setCursor(Qt::PointingHandCursor);
    }

    int low() const { return low_; }
    int high() const { return high_; }

    void setRange(int low, int high)
    {
        low_ = std::clamp(low, 0, 255);
        high_ = std::clamp(high, 0, 255);
        if (low_ > high_) {
            std::swap(low_, high_);
        }
        update();
    }

    std::function<void(int, int)> changed;

    QSize sizeHint() const override { return {200, 20}; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing, true);
        const QRectF groove = grooveRect();
        QLinearGradient ramp(groove.topLeft(), groove.topRight());
        ramp.setColorAt(0.0, Qt::black);
        ramp.setColorAt(1.0, Qt::white);
        painter.setPen(QPen(QColor(90, 90, 90), 1));
        painter.setBrush(ramp);
        painter.drawRect(groove);
        const auto handle = [&](int value, const QColor& color) {
            const qreal x = handleX(value);
            QPainterPath path;
            path.moveTo(x, groove.bottom() + 1.0);
            path.lineTo(x - 5.0, groove.bottom() + 7.0);
            path.lineTo(x + 5.0, groove.bottom() + 7.0);
            path.closeSubpath();
            painter.setBrush(isEnabled() ? color : color.darker(160));
            painter.setPen(QPen(QColor(60, 60, 60), 1));
            painter.drawPath(path);
        };
        handle(low_, Qt::black);
        handle(high_, Qt::white);
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (!isEnabled()) {
            return;
        }
        const int x = int(event->position().x());
        dragging_ = std::abs(x - handleX(low_)) <= std::abs(x - handleX(high_)) ? 0 : 1;
        mouseMoveEvent(event);
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (dragging_ < 0) {
            return;
        }
        const int value = valueAt(int(event->position().x()));
        if (dragging_ == 0) {
            low_ = std::min(value, high_);
        } else {
            high_ = std::max(value, low_);
        }
        update();
        if (changed) {
            changed(low_, high_);
        }
    }

    void mouseReleaseEvent(QMouseEvent*) override { dragging_ = -1; }

private:
    QRectF grooveRect() const { return QRectF(8.0, 1.0, width() - 16.0, 8.0); }
    int handleX(int value) const { return 8 + value * (width() - 16) / 255; }
    int valueAt(int x) const
    {
        return std::clamp((x - 8) * 255 / std::max(1, width() - 16), 0, 255);
    }

    int low_ = 40;
    int high_ = 118;
    int dragging_ = -1;
};

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
    : QDialog(parent), view_(view), spec_(spec), iris_(spec.kind == QStringLiteral("iris-blur"))
{
    setWindowTitle(spec.label);
    setObjectName(QStringLiteral("blurGalleryDialog"));
    setMinimumSize(760, 520);
    const QWidget* frame = parent ? parent->window() : nullptr;
    resize(frame ? (QSizeF(frame->size()) * 0.92).toSize().expandedTo(minimumSize())
                 : QSize(1100, 720));

    canvas_ = new BlurGalleryCanvas(&pins_, this);
    canvas_->setIris(iris_);
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
            if (view_->document_width() > 0) {
                proxyScale_ = double(proxy_.width()) / double(view_->document_width());
            }
            canvas_->setDocumentSize(QSize(view_->document_width(), view_->document_height()));
        }
    }
    pins_.append(defaultPin());
    connect(canvas_, &BlurGalleryCanvas::pinsEdited, this, &BlurGalleryDialog::pinsEdited);
    connect(canvas_, &BlurGalleryCanvas::pinSelected, this, &BlurGalleryDialog::selectPin);
    connect(canvas_, &BlurGalleryCanvas::pinAdded, this, [this](int index) { selectPin(index); });
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
    outer->addLayout(body, 1);
    outer->addWidget(buildOptionsBar());

    auto* hide = new QShortcut(QKeySequence(Qt::Key_H), this);
    hide->setContext(Qt::WidgetWithChildrenShortcut);
    connect(hide, &QShortcut::activated, this,
            [this] { canvas_->setUiVisible(!canvas_->uiVisible()); });

    syncControls();
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
    title->setStyleSheet(QStringLiteral("font-weight: 700;"));
    row->addWidget(title);
    row->addSpacing(12);
    auto* hint = new QLabel(
        iris_ ? QStringLiteral("Click to add an iris; drag its handles to size, turn, feather, "
                               "or square it; H hides the overlay")
              : QStringLiteral("Click the image to add a pin, drag a pin to move it, its ring to "
                               "blur, H to hide the pins"),
        bar);
    hint->setEnabled(false);
    row->addWidget(hint);
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
                                      const QString& checkName, bool checked, bool enabled,
                                      QToolButton** arrow)
{
    auto* row = new QWidget(parent);
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);
    auto* triangle = new QToolButton(row);
    triangle->setObjectName(checkName + QStringLiteral("Arrow"));
    triangle->setAutoRaise(true);
    triangle->setText(checked ? QStringLiteral("▼") : QStringLiteral("▶"));
    triangle->setEnabled(enabled);
    auto* name = new QLabel(label, row);
    auto* check = new QCheckBox(row);
    check->setObjectName(checkName);
    check->setChecked(checked);
    check->setEnabled(enabled);
    layout->addWidget(triangle);
    layout->addWidget(name, 1);
    layout->addWidget(check);
    if (arrow) {
        *arrow = triangle;
    }
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
    tab->setStyleSheet(QStringLiteral("font-weight: 700; padding: 3px 0;"));
    layout->addWidget(tab);

    const struct {
        const char* label;
        const char* name;
        const char* kind;
    } rows[] = {
        {"Field Blur", "fieldBlur", "field-blur"}, {"Iris Blur", "irisBlur", "iris-blur"},
        {"Tilt-Shift", "tiltShift", nullptr},      {"Path Blur", "pathBlur", nullptr},
        {"Spin Blur", "spinBlur", nullptr},
    };
    for (const auto& row : rows) {
        const QString name = QLatin1String(row.name);
        const bool active = row.kind && spec_.kind == QLatin1String(row.kind);
        auto* widget = effectRow(panel, QLatin1String(row.label), name + QStringLiteral("Enabled"),
                                 active, active, active ? &toolArrow_ : nullptr);
        layout->addWidget(widget);
        if (!active) {
            widget->setToolTip(row.kind ? QStringLiteral("Not combined with this blur yet")
                                        : QStringLiteral("Not implemented yet"));
            continue;
        }
        toolControls_ = new QWidget(panel);
        auto* controls = new QVBoxLayout(toolControls_);
        controls->setContentsMargins(18, 0, 0, 0);
        controls->setSpacing(3);
        auto* blurRow = new QHBoxLayout;
        auto* label = new QLabel(QStringLiteral("Blur:"), toolControls_);
        blurSpin_ = makeValueBox(toolControls_, name + QStringLiteral("Spin"), 0.0, kMaxBlur, 0.0);
        blurSpin_->setSuffix(QStringLiteral(" px"));
        label->setBuddy(blurSpin_);
        blurRow->addWidget(label);
        blurRow->addStretch(1);
        blurRow->addWidget(blurSpin_);
        controls->addLayout(blurRow);
        blurSlider_ = new JumpSlider(Qt::Horizontal, toolControls_);
        blurSlider_->setObjectName(name + QStringLiteral("Slider"));
        blurSlider_->setRange(0, int(kMaxBlur));
        controls->addWidget(blurSlider_);
        layout->addWidget(toolControls_);
    }
    connect(blurSpin_, &QDoubleSpinBox::valueChanged, this,
            [this](double value) { setSelectedBlur(value); });
    connect(blurSlider_, &QSlider::valueChanged, this,
            [this](int value) { setSelectedBlur(value); });
    connect(toolArrow_, &QToolButton::clicked, this, [this] {
        const bool show = !toolControls_->isVisible();
        toolControls_->setVisible(show);
        toolArrow_->setText(show ? QStringLiteral("▼") : QStringLiteral("▶"));
    });
    return panel;
}

QWidget* BlurGalleryDialog::buildEffectsPanel()
{
    auto* panel = new QFrame(this);
    panel->setObjectName(QStringLiteral("blurEffectsPanel"));
    panel->setFrameShape(QFrame::StyledPanel);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(2, 2, 2, 2);
    auto* tabs = new QTabWidget(panel);
    tabs->setObjectName(QStringLiteral("blurEffectsTabs"));
    tabs->addTab(buildBokehTab(), QStringLiteral("Effects"));
    tabs->addTab(buildMotionTab(), QStringLiteral("Motion Effects"));
    tabs->addTab(buildNoiseTab(), QStringLiteral("Noise"));
    layout->addWidget(tabs);
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
    name->setStyleSheet(QStringLiteral("font-weight: 700;"));
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

QWidget* BlurGalleryDialog::buildMotionTab()
{
    auto* tab = new QWidget(this);
    auto* layout = new QVBoxLayout(tab);
    layout->setContentsMargins(6, 6, 6, 6);
    auto* hint = new QLabel(
        QStringLiteral("Motion Effects apply to Path Blur and Spin Blur, which are not "
                       "implemented yet."),
        tab);
    hint->setWordWrap(true);
    hint->setEnabled(false);
    layout->addWidget(hint);
    layout->addStretch(1);
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

QList<double> BlurGalleryDialog::values() const
{
    QList<double> out;
    out << pins_.size();
    for (const BlurGalleryPin& pin : pins_) {
        out << pin.pos.x() << pin.pos.y();
        if (iris_) {
            out << pin.radii.width() << pin.radii.height() << pin.angle << pin.squareness
                << pin.feather;
        }
        out << pin.blur;
    }
    out << (bokehEnabled_->isChecked() ? 1.0 : 0.0) << bokehLight_.spin->value()
        << bokehColor_.spin->value() << bokehRangeLow_->value() << bokehRangeHigh_->value()
        << (noiseEnabled_->isChecked() ? 1.0 : 0.0) << noiseAmount_.spin->value()
        << noiseSize_.spin->value() << noiseRoughness_.spin->value() << noiseColor_.spin->value()
        << noiseHighlights_.spin->value() << double(noiseKind_->currentIndex()) << noiseSeed_;
    return out;
}

void BlurGalleryDialog::setValues(const QList<double>& values)
{
    QList<BlurGalleryPin> pins;
    const int count = values.isEmpty() ? 0 : int(values.first());
    const int stride = pinSlots();
    const bool whole = !values.isEmpty() && double(count) == values.first() && count >= 1
                       && values.size() == 1 + stride * count + kEffectsTail;
    if (whole) {
        for (int i = 0; i < count; ++i) {
            const auto slot = [&](int k) { return values.at(1 + stride * i + k); };
            BlurGalleryPin pin;
            pin.pos = QPointF(slot(0), slot(1));
            if (iris_) {
                pin.radii = QSizeF(qBound(kMinIrisRadius, slot(2), kMaxIrisRadius),
                                   qBound(kMinIrisRadius, slot(3), kMaxIrisRadius));
                pin.angle = slot(4);
                pin.squareness = qBound(0.0, slot(5), 100.0);
                pin.feather = qBound(0.0, slot(6), 1.0);
            }
            pin.blur = qBound(0.0, slot(stride - 1), kMaxBlur);
            pins.append(pin);
        }
        const int tail = 1 + stride * count;
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
    if (pins.isEmpty()) {
        pins.append(defaultPin());
    }
    pins_ = pins;
    selectPin(0);
    effectsEdited();
}

int BlurGalleryDialog::pinSlots() const
{
    return iris_ ? 8 : 3;
}

BlurGalleryPin BlurGalleryDialog::defaultPin() const
{
    BlurGalleryPin pin;
    if (!iris_) {
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

void BlurGalleryDialog::selectPin(int index)
{
    if (index < 0 || index >= pins_.size()) {
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
    const int index = canvas_->selected();
    if (pins_.size() <= 1 || index < 0 || index >= pins_.size()) {
        return;
    }
    pins_.removeAt(index);
    canvas_->setSelected(qMin(index, int(pins_.size()) - 1));
    syncControls();
    canvas_->update();
    scheduleRender();
}

void BlurGalleryDialog::setSelectedBlur(double blur)
{
    const int index = canvas_->selected();
    if (index < 0 || index >= pins_.size()) {
        return;
    }
    pins_[index].blur = qBound(0.0, blur, kMaxBlur);
    {
        const QSignalBlocker blockSpin(blurSpin_);
        const QSignalBlocker blockSlider(blurSlider_);
        blurSpin_->setValue(pins_[index].blur);
        blurSlider_->setValue(qRound(pins_[index].blur));
    }
    canvas_->update();
    scheduleRender();
}

void BlurGalleryDialog::syncControls()
{
    const int index = canvas_->selected();
    const bool has = index >= 0 && index < pins_.size();
    blurSpin_->setEnabled(has);
    blurSlider_->setEnabled(has);
    if (!has) {
        return;
    }
    const QSignalBlocker blockSpin(blurSpin_);
    const QSignalBlocker blockSlider(blurSlider_);
    blurSpin_->setValue(pins_.at(index).blur);
    blurSlider_->setValue(qRound(pins_.at(index).blur));
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
    QList<double> params = values();
    const int stride = pinSlots();
    const int pinEnd = 1 + stride * qRound(params.value(0));
    for (int i = stride; i < pinEnd; i += stride) {
        params[i] *= proxyScale_;
    }
    const QImage filtered =
        filter_thumbnail(proxyRgba_, proxy_.width(), proxy_.height(), spec_.kind, params);
    canvas_->setImage(filtered.isNull() ? proxy_ : filtered);
}

bool BlurGalleryDialog::get(PictureView* view, const FilterCommandSpec& spec,
                            const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    BlurGalleryDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura
