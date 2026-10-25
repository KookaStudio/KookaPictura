#include "color_picker_dialog.h"
#include "color_picker_math.h"
#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "dialogs.h"
#include "panels/color_swatch_widget.h"
#include "panels/swatches_panel.h"
#include "tools.h"

#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QRegularExpressionValidator>
#include <QtGui/QResizeEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <array>
#include <cmath>

namespace pictura {

namespace {

/// Photoshop's field is 256x256 logical pixels.
constexpr int kPlaneSize = 256;
/// Width of the ramp's colour strip, excluding the arrow gutters.
constexpr int kRampWidth = 19;
/// Gutter either side of the strip, where the arrow markers are drawn.
constexpr int kRampGutter = 7;

// -- axis mapping -----------------------------------------------------------
//
//   axis   ramp          field X       field Y (top -> bottom)
//   ----   -----------   -----------   ---------------------
//   H      hue           saturation    brightness  100 -> 0
//   S      saturation    hue           brightness  100 -> 0
//   B      brightness    hue           saturation  100 -> 0
//   R      red           blue          green       255 -> 0
//   G      green         blue          red         255 -> 0
//   B      blue          red           green       255 -> 0

QColor planeColorAt(ColorAxis axis, int h, int s, int v, double x, double y)
{
    const auto to255 = [](double t) { return int(qBound(0.0, t, 1.0) * 255.0 + 0.5); };
    const auto to359 = [](double t) { return int(qBound(0.0, t, 1.0) * 359.0 + 0.5); };
    const QColor base = QColor::fromHsv(h, s, v);

    switch (axis) {
    case ColorAxis::Hue:
        return QColor::fromHsv(h, to255(x), to255(1.0 - y));
    case ColorAxis::Saturation:
        return QColor::fromHsv(to359(x), s, to255(1.0 - y));
    case ColorAxis::Brightness:
        return QColor::fromHsv(to359(x), to255(1.0 - y), v);
    case ColorAxis::Red:
        return QColor(base.red(), to255(1.0 - y), to255(x));
    case ColorAxis::Green:
        return QColor(to255(1.0 - y), base.green(), to255(x));
    case ColorAxis::Blue:
        return QColor(to255(x), to255(1.0 - y), base.blue());
    }
    return base;
}

QPointF planePosFor(ColorAxis axis, int h, int s, int v)
{
    const QColor c = QColor::fromHsv(h, s, v);
    switch (axis) {
    case ColorAxis::Hue:
        return {s / 255.0, 1.0 - v / 255.0};
    case ColorAxis::Saturation:
        return {h / 359.0, 1.0 - v / 255.0};
    case ColorAxis::Brightness:
        return {h / 359.0, 1.0 - s / 255.0};
    case ColorAxis::Red:
        return {c.blue() / 255.0, 1.0 - c.green() / 255.0};
    case ColorAxis::Green:
        return {c.blue() / 255.0, 1.0 - c.red() / 255.0};
    case ColorAxis::Blue:
        return {c.red() / 255.0, 1.0 - c.green() / 255.0};
    }
    return {0.0, 0.0};
}

/// Colour along the ramp. `t` is 0 at the bottom, 1 at the top. The hue ramp
/// runs red -> magenta -> blue -> cyan -> green -> yellow -> red downward.
QColor rampColorAt(ColorAxis axis, int h, int s, int v, double t)
{
    const auto to255 = [](double x) { return int(qBound(0.0, x, 1.0) * 255.0 + 0.5); };
    const QColor c = QColor::fromHsv(h, s, v);

    switch (axis) {
    case ColorAxis::Hue:
        return QColor::fromHsv(int(qBound(0.0, t, 1.0) * 359.0 + 0.5), 255, 255);
    case ColorAxis::Saturation:
        return QColor::fromHsv(h, to255(t), v);
    case ColorAxis::Brightness:
        return QColor::fromHsv(h, s, to255(t));
    case ColorAxis::Red:
        return QColor(to255(t), c.green(), c.blue());
    case ColorAxis::Green:
        return QColor(c.red(), to255(t), c.blue());
    case ColorAxis::Blue:
        return QColor(c.red(), c.green(), to255(t));
    }
    return c;
}

double rampPosFor(ColorAxis axis, int h, int s, int v)
{
    const QColor c = QColor::fromHsv(h, s, v);
    switch (axis) {
    case ColorAxis::Hue:
        return h / 359.0;
    case ColorAxis::Saturation:
        return s / 255.0;
    case ColorAxis::Brightness:
        return v / 255.0;
    case ColorAxis::Red:
        return c.red() / 255.0;
    case ColorAxis::Green:
        return c.green() / 255.0;
    case ColorAxis::Blue:
        return c.blue() / 255.0;
    }
    return 0.0;
}

int planeCacheKey(ColorAxis axis, int h, int s, int v)
{
    const QColor c = QColor::fromHsv(h, s, v);
    switch (axis) {
    case ColorAxis::Hue:
        return h;
    case ColorAxis::Saturation:
        return s;
    case ColorAxis::Brightness:
        return v;
    case ColorAxis::Red:
        return c.red();
    case ColorAxis::Green:
        return c.green();
    case ColorAxis::Blue:
        return c.blue();
    }
    return 0;
}

/// Convert to HSV while keeping `h` when the colour is achromatic.
void drawPlaneMarker(QPainter& p, const QPointF& pos)
{
    p.setBrush(Qt::NoBrush);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setPen(QPen(QColor(0, 0, 0, 190), 1.0));
    p.drawEllipse(pos, 5.5, 5.5);
    p.setPen(QPen(QColor(255, 255, 255, 230), 1.5));
    p.drawEllipse(pos, 4.5, 4.5);
}

// -- Add to Swatches --------------------------------------------------------

QPointer<SwatchGrid> g_swatchGrid;
int g_pickerChoice = 0;

bool promptForSwatchName(QWidget* parent, const QColor& color, const QString& suggested,
                         QString* name)
{
    QDialog dialog(parent);
    dialog.setObjectName(QStringLiteral("swatchNameDialog"));
    dialog.setWindowTitle(QObject::tr("Color Swatch Name"));

    auto* root = new QHBoxLayout(&dialog);
    root->setContentsMargins(12, 12, 12, 12);
    root->setSpacing(10);

    auto* preview = new ColorSwatchWidget(&dialog);
    preview->setObjectName(QStringLiteral("swatchNamePreview"));
    preview->setFixedSize(44, 44);
    preview->setFlat(true);
    preview->setFocusPolicy(Qt::NoFocus);
    preview->setColor(color);
    root->addWidget(preview, 0, Qt::AlignTop);

    auto* right = new QVBoxLayout();
    auto* nameRow = new QHBoxLayout();
    nameRow->addWidget(new QLabel(QObject::tr("Name:"), &dialog));
    auto* edit = new QLineEdit(suggested, &dialog);
    edit->setObjectName(QStringLiteral("swatchNameEdit"));
    edit->setMinimumWidth(200);
    nameRow->addWidget(edit, 1);
    right->addLayout(nameRow);
    right->addStretch(1);

    auto* buttons = new QHBoxLayout();
    buttons->addStretch(1);
    auto* ok = new QPushButton(QObject::tr("OK"), &dialog);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QObject::tr("Cancel"), &dialog);
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    right->addLayout(buttons);
    root->addLayout(right, 1);

    QObject::connect(ok, &QPushButton::clicked, &dialog, &QDialog::accept);
    QObject::connect(cancel, &QPushButton::clicked, &dialog, &QDialog::reject);
    edit->selectAll();

    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *name = edit->text().trimmed();
    return true;
}

} // namespace

// ===========================================================================
// ColorPlane
// ===========================================================================

ColorPlane::ColorPlane(QWidget* parent)
    : QWidget(parent)
{
    setCursor(Qt::CrossCursor);
    // Flexible so the inline picker in the Color panel can shrink to the dock;
    // the dialog gives it its 256px sizeHint.
    setMinimumSize(120, 120);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
}

QSize ColorPlane::sizeHint() const
{
    return QSize(kPlaneSize, kPlaneSize);
}

void ColorPlane::setAxis(ColorAxis axis)
{
    if (m_axis == axis) {
        return;
    }
    m_axis = axis;
    update();
}

void ColorPlane::setHsv(int hue, int sat, int val)
{
    m_hue = hue;
    m_sat = sat;
    m_val = val;
    update();
}

void ColorPlane::setWebColorsOnly(bool webOnly)
{
    if (m_webOnly == webOnly) {
        return;
    }
    m_webOnly = webOnly;
    update();
}

void ColorPlane::rebuildCache()
{
    const int key = planeCacheKey(m_axis, m_hue, m_sat, m_val);
    if (!m_cache.isNull() && m_cacheAxis == m_axis && m_cacheValue == key
        && m_cacheWebOnly == m_webOnly && m_cache.size() == size()) {
        return;
    }

    m_cache = QImage(size(), QImage::Format_RGB32);
    const int w = width();
    const int h = height();
    if (w <= 0 || h <= 0) {
        return;
    }

    for (int y = 0; y < h; ++y) {
        auto* line = reinterpret_cast<QRgb*>(m_cache.scanLine(y));
        const double yn = h > 1 ? y / double(h - 1) : 0.0;
        for (int x = 0; x < w; ++x) {
            const double xn = w > 1 ? x / double(w - 1) : 0.0;
            QColor c = planeColorAt(m_axis, m_hue, m_sat, m_val, xn, yn);
            if (m_webOnly) {
                c = snapToWebColor(c);
            }
            line[x] = c.rgb();
        }
    }

    m_cacheAxis = m_axis;
    m_cacheValue = key;
    m_cacheWebOnly = m_webOnly;
}

void ColorPlane::paintEvent(QPaintEvent*)
{
    rebuildCache();

    QPainter p(this);
    p.drawImage(0, 0, m_cache);

    const QPointF pos = planePosFor(m_axis, m_hue, m_sat, m_val);
    drawPlaneMarker(p, QPointF(pos.x() * (width() - 1), pos.y() * (height() - 1)));

    p.setRenderHint(QPainter::Antialiasing, false);
    p.setPen(QColor(0x2a, 0x2a, 0x2a));
    p.setBrush(Qt::NoBrush);
    p.drawRect(rect().adjusted(0, 0, -1, -1));
}

void ColorPlane::pickAt(const QPoint& pos)
{
    const double xn = width() > 1 ? qBound(0.0, pos.x() / double(width() - 1), 1.0) : 0.0;
    const double yn = height() > 1 ? qBound(0.0, pos.y() / double(height() - 1), 1.0) : 0.0;

    QColor c = planeColorAt(m_axis, m_hue, m_sat, m_val, xn, yn);
    if (m_webOnly) {
        c = snapToWebColor(c);
    }

    int h = m_hue;
    int s = m_sat;
    int v = m_val;
    toHsvPreservingHue(c, h, s, v);
    emit picked(h, s, v);
}

void ColorPlane::mousePressEvent(QMouseEvent* event)
{
    pickAt(event->pos());
}

void ColorPlane::mouseMoveEvent(QMouseEvent* event)
{
    if (event->buttons() & Qt::LeftButton) {
        pickAt(event->pos());
    }
}

void ColorPlane::resizeEvent(QResizeEvent*)
{
    m_cache = QImage();
}

// ===========================================================================
// ColorRamp
// ===========================================================================

ColorRamp::ColorRamp(QWidget* parent)
    : QWidget(parent)
{
    setCursor(Qt::SizeVerCursor);
    setMinimumWidth(kRampWidth + kRampGutter * 2);
    setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Expanding);
}

QSize ColorRamp::sizeHint() const
{
    return QSize(kRampWidth + kRampGutter * 2, kPlaneSize);
}

QRect ColorRamp::stripRect() const
{
    return QRect(kRampGutter, 0, kRampWidth, height());
}

void ColorRamp::setAxis(ColorAxis axis)
{
    if (m_axis == axis) {
        return;
    }
    m_axis = axis;
    update();
}

void ColorRamp::setHsv(int hue, int sat, int val)
{
    m_hue = hue;
    m_sat = sat;
    m_val = val;
    update();
}

void ColorRamp::setWebColorsOnly(bool webOnly)
{
    if (m_webOnly == webOnly) {
        return;
    }
    m_webOnly = webOnly;
    update();
}

void ColorRamp::rebuildCache()
{
    const bool valid = !m_cache.isNull() && m_cacheAxis == m_axis
        && m_cacheWebOnly == m_webOnly && m_cache.height() == height()
        && (m_axis == ColorAxis::Hue
            || (m_cacheHue == m_hue && m_cacheSat == m_sat && m_cacheVal == m_val));
    if (valid) {
        return;
    }

    const QRect strip = stripRect();
    if (strip.width() <= 0 || strip.height() <= 0) {
        return;
    }
    m_cache = QImage(strip.size(), QImage::Format_RGB32);

    for (int y = 0; y < strip.height(); ++y) {
        const double t = strip.height() > 1 ? 1.0 - y / double(strip.height() - 1) : 0.0;
        QColor c = rampColorAt(m_axis, m_hue, m_sat, m_val, t);
        if (m_webOnly) {
            c = snapToWebColor(c);
        }
        auto* line = reinterpret_cast<QRgb*>(m_cache.scanLine(y));
        for (int x = 0; x < strip.width(); ++x) {
            line[x] = c.rgb();
        }
    }

    m_cacheAxis = m_axis;
    m_cacheHue = m_hue;
    m_cacheSat = m_sat;
    m_cacheVal = m_val;
    m_cacheWebOnly = m_webOnly;
}

void ColorRamp::paintEvent(QPaintEvent*)
{
    rebuildCache();

    QPainter p(this);
    const QRect strip = stripRect();
    p.drawImage(strip.topLeft(), m_cache);
    p.setPen(QColor(0x2a, 0x2a, 0x2a));
    p.setBrush(Qt::NoBrush);
    p.drawRect(strip.adjusted(0, 0, -1, -1));

    const double t = rampPosFor(m_axis, m_hue, m_sat, m_val);
    const double y = (1.0 - t) * (strip.height() - 1);

    p.setRenderHint(QPainter::Antialiasing, false);
    p.setPen(Qt::NoPen);
    p.setBrush(palette().color(QPalette::WindowText));

    const QPointF left[3] = {
        {0.0, y - 4.0},
        {double(kRampGutter) - 1.0, y},
        {0.0, y + 4.0},
    };
    p.drawPolygon(left, 3);

    const double rightEdge = width();
    const QPointF right[3] = {
        {rightEdge, y - 4.0},
        {rightEdge - kRampGutter + 1.0, y},
        {rightEdge, y + 4.0},
    };
    p.drawPolygon(right, 3);
}

void ColorRamp::pickAt(const QPoint& pos)
{
    const QRect strip = stripRect();
    const double t =
        strip.height() > 1 ? qBound(0.0, 1.0 - pos.y() / double(strip.height() - 1), 1.0) : 0.0;

    QColor c = rampColorAt(m_axis, m_hue, m_sat, m_val, t);
    if (m_webOnly) {
        c = snapToWebColor(c);
    }

    int h = m_hue;
    int s = m_sat;
    int v = m_val;
    if (m_axis == ColorAxis::Hue) {
        h = int(qBound(0.0, t, 1.0) * 359.0 + 0.5);
    } else {
        toHsvPreservingHue(c, h, s, v);
    }
    emit picked(h, s, v);
}

void ColorRamp::mousePressEvent(QMouseEvent* event)
{
    pickAt(event->pos());
}

void ColorRamp::mouseMoveEvent(QMouseEvent* event)
{
    if (event->buttons() & Qt::LeftButton) {
        pickAt(event->pos());
    }
}

void ColorRamp::resizeEvent(QResizeEvent*)
{
    m_cache = QImage();
}

// ===========================================================================
// ColorCompare
// ===========================================================================

ColorCompare::ColorCompare(QWidget* parent)
    : QWidget(parent)
{
    setFixedSize(60, 52);
    setToolTip(tr("Click the lower swatch to restore the original color"));
}

QSize ColorCompare::sizeHint() const
{
    return QSize(60, 52);
}

void ColorCompare::setCurrentColor(const QColor& color)
{
    m_current = color;
    update();
}

void ColorCompare::setOriginalColor(const QColor& color)
{
    m_original = color;
    update();
}

void ColorCompare::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const int half = height() / 2;
    p.fillRect(QRect(0, 0, width(), half), m_current);
    p.fillRect(QRect(0, half, width(), height() - half), m_original);

    p.setPen(QColor(0x2a, 0x2a, 0x2a));
    p.setBrush(Qt::NoBrush);
    p.drawRect(rect().adjusted(0, 0, -1, -1));
}

void ColorCompare::mousePressEvent(QMouseEvent* event)
{
    if (event->pos().y() >= height() / 2) {
        emit originalClicked();
    }
}

// ===========================================================================
// ColorPicker
// ===========================================================================

ColorPicker::ColorPicker(QWidget* parent)
    : QWidget(parent)
{
    buildUi();
    setColor(QColor(Qt::black));
    m_compare->setOriginalColor(m_color);
}

void ColorPicker::buildUi()
{
    auto* root = new QHBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(10);

    auto* fieldColumn = new QVBoxLayout();
    fieldColumn->setSpacing(8);
    m_plane = new ColorPlane(this);
    fieldColumn->addWidget(m_plane);

    m_webOnly = new QCheckBox(tr("Only Web Colors"), this);
    m_webOnly->setObjectName(QStringLiteral("colorPickerOnlyWeb"));
    m_webOnly->setToolTip(tr("Restrict the picker to the 216-color web-safe palette"));
    fieldColumn->addWidget(m_webOnly);
    fieldColumn->addStretch(1);
    root->addLayout(fieldColumn);

    auto* rampColumn = new QVBoxLayout();
    rampColumn->setSpacing(8);
    m_ramp = new ColorRamp(this);
    rampColumn->addWidget(m_ramp);
    rampColumn->addStretch(1);
    root->addLayout(rampColumn);

    auto* centre = new QVBoxLayout();
    centre->setSpacing(6);

    auto* newLabel = new QLabel(tr("new"), this);
    newLabel->setAlignment(Qt::AlignHCenter);
    centre->addWidget(newLabel);

    m_compare = new ColorCompare(this);
    centre->addWidget(m_compare, 0, Qt::AlignHCenter);

    auto* currentLabel = new QLabel(tr("current"), this);
    currentLabel->setAlignment(Qt::AlignHCenter);
    centre->addWidget(currentLabel);
    centre->addSpacing(8);

    m_webAlert = new QPushButton(tr("Web-safe"), this);
    m_webAlert->setObjectName(QStringLiteral("colorPickerWebAlert"));
    m_webAlert->setToolTip(tr("Click to select the closest web-safe color"));
    m_webAlert->setVisible(false);
    centre->addWidget(m_webAlert);
    m_gamutAlert = new QPushButton(tr("Out of gamut"), this);
    m_gamutAlert->setObjectName(QStringLiteral("colorPickerGamutAlert"));
    m_gamutAlert->setToolTip(tr("Click to select the closest printable color"));
    m_gamutAlert->setVisible(false);
    centre->addWidget(m_gamutAlert);
    connect(m_webAlert, &QPushButton::clicked, this, [this]() {
        setColor(snapToWebColor(m_color));
        notify();
    });
    connect(m_gamutAlert, &QPushButton::clicked, this, [this]() {
        const uint32_t rgb = pictura::picker_closest_rgb(m_color.red(), m_color.green(),
                                                         m_color.blue());
        if (rgb != 0xFFFFFFFFu) {
            setColor(QColor::fromRgb(QRgb(rgb)));
            notify();
        }
    });

    auto* grid = new QGridLayout();
    grid->setHorizontalSpacing(4);
    grid->setVerticalSpacing(3);

    const auto addRow = [&](int row, int col, QRadioButton** radio, const QString& label,
                            QSpinBox** spin, int lo, int hi, const QString& unit, bool readOnly) {
        if (radio) {
            *radio = new QRadioButton(label, this);
            grid->addWidget(*radio, row, col);
        } else {
            auto* text = new QLabel(label, this);
            grid->addWidget(text, row, col, Qt::AlignRight | Qt::AlignVCenter);
        }
        *spin = new QSpinBox(this);
        (*spin)->setRange(lo, hi);
        (*spin)->setFixedWidth(52);
        (*spin)->setButtonSymbols(QAbstractSpinBox::NoButtons);
        (*spin)->setReadOnly(readOnly);
        if (readOnly) {
            (*spin)->setFocusPolicy(Qt::NoFocus);
        }
        grid->addWidget(*spin, row, col + 1);
        grid->addWidget(new QLabel(unit, this), row, col + 2);
    };

    addRow(0, 0, &m_radioH, tr("H:"), &m_spinH, 0, 360, tr("°"), false);
    addRow(1, 0, &m_radioS, tr("S:"), &m_spinS, 0, 100, tr("%"), false);
    addRow(2, 0, &m_radioB, tr("B:"), &m_spinB, 0, 100, tr("%"), false);
    addRow(3, 0, &m_radioR, tr("R:"), &m_spinR, 0, 255, QString(), false);
    addRow(4, 0, &m_radioG, tr("G:"), &m_spinG, 0, 255, QString(), false);
    addRow(5, 0, &m_radioBlue, tr("B:"), &m_spinBlue, 0, 255, QString(), false);

    addRow(0, 4, nullptr, tr("L:"), &m_spinL, 0, 100, QString(), false);
    addRow(1, 4, nullptr, tr("a:"), &m_spinLabA, -128, 127, QString(), false);
    addRow(2, 4, nullptr, tr("b:"), &m_spinLabB, -128, 127, QString(), false);
    addRow(3, 4, nullptr, tr("C:"), &m_spinC, 0, 100, tr("%"), false);
    addRow(4, 4, nullptr, tr("M:"), &m_spinM, 0, 100, tr("%"), false);
    addRow(5, 4, nullptr, tr("Y:"), &m_spinY, 0, 100, tr("%"), false);
    addRow(6, 4, nullptr, tr("K:"), &m_spinK, 0, 100, tr("%"), false);
    m_spinL->setObjectName(QStringLiteral("colorPickerL"));
    m_spinLabA->setObjectName(QStringLiteral("colorPickerLabA"));
    m_spinLabB->setObjectName(QStringLiteral("colorPickerLabB"));
    m_spinC->setObjectName(QStringLiteral("colorPickerC"));
    m_spinM->setObjectName(QStringLiteral("colorPickerM"));
    m_spinY->setObjectName(QStringLiteral("colorPickerY"));
    m_spinK->setObjectName(QStringLiteral("colorPickerK"));

    const QString cmykTip =
        tr("CMYK is projected through the working CMYK profile from Color Settings.");
    // ponytail: the picker fields are 8-bit; 16/32-bit picker fields remain
    // #604's scope.
    for (QSpinBox* box : {m_spinC, m_spinM, m_spinY, m_spinK}) {
        box->setToolTip(cmykTip);
    }
    for (QSpinBox* box : {m_spinL, m_spinLabA, m_spinLabB}) {
        box->setToolTip(tr("CIE L*a*b*, D50 reference white"));
    }

    grid->addWidget(new QLabel(QStringLiteral("#"), this), 6, 0,
                    Qt::AlignRight | Qt::AlignVCenter);
    m_hex = new QLineEdit(this);
    m_hex->setObjectName(QStringLiteral("colorPickerHex"));
    m_hex->setFixedWidth(84);
    m_hex->setMaxLength(8);
    m_hex->setValidator(new QRegularExpressionValidator(
        QRegularExpression(QStringLiteral("(?:#|0[xX])?[0-9A-Fa-f]{0,6}")), this));
    grid->addWidget(m_hex, 6, 1, 1, 2);

    grid->setColumnMinimumWidth(3, 14);
    centre->addLayout(grid);
    centre->addStretch(1);
    root->addLayout(centre);

    connect(m_plane, &ColorPlane::picked, this, &ColorPicker::onPlanePicked);
    connect(m_ramp, &ColorRamp::picked, this, &ColorPicker::onPlanePicked);
    connect(m_compare, &ColorCompare::originalClicked, this, &ColorPicker::revertToOriginal);

    for (QRadioButton* r : {m_radioH, m_radioS, m_radioB, m_radioR, m_radioG, m_radioBlue}) {
        connect(r, &QRadioButton::toggled, this, &ColorPicker::onAxisChanged);
    }
    for (QSpinBox* s : {m_spinH, m_spinS, m_spinB}) {
        connect(s, &QSpinBox::valueChanged, this, &ColorPicker::onHsbFieldsEdited);
    }
    for (QSpinBox* s : {m_spinR, m_spinG, m_spinBlue}) {
        connect(s, &QSpinBox::valueChanged, this, &ColorPicker::onRgbFieldsEdited);
    }
    for (QSpinBox* s : {m_spinL, m_spinLabA, m_spinLabB}) {
        connect(s, &QSpinBox::valueChanged, this, &ColorPicker::onLabFieldsEdited);
    }
    for (QSpinBox* s : {m_spinC, m_spinM, m_spinY, m_spinK}) {
        connect(s, &QSpinBox::valueChanged, this, &ColorPicker::onCmykFieldsEdited);
    }
    connect(m_hex, &QLineEdit::textEdited, this, &ColorPicker::onHexEdited);
    connect(m_hex, &QLineEdit::editingFinished, this, &ColorPicker::onHexCommitted);
    connect(m_webOnly, &QCheckBox::toggled, this, &ColorPicker::onWebColorsToggled);

    m_radioH->setChecked(true);
}

ColorAxis ColorPicker::currentAxis() const
{
    if (m_radioS->isChecked()) {
        return ColorAxis::Saturation;
    }
    if (m_radioB->isChecked()) {
        return ColorAxis::Brightness;
    }
    if (m_radioR->isChecked()) {
        return ColorAxis::Red;
    }
    if (m_radioG->isChecked()) {
        return ColorAxis::Green;
    }
    if (m_radioBlue->isChecked()) {
        return ColorAxis::Blue;
    }
    return ColorAxis::Hue;
}

void ColorPicker::setHsv(int hue, int sat, int val)
{
    m_hue = qBound(0, hue, 359);
    m_sat = qBound(0, sat, 255);
    m_val = qBound(0, val, 255);
    m_color = QColor::fromHsv(m_hue, m_sat, m_val);
}

void ColorPicker::setColor(const QColor& color)
{
    m_color = color.toRgb();
    toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    syncControls();
}

void ColorPicker::setOriginalColor(const QColor& color)
{
    m_original = color;
    if (m_compare) {
        m_compare->setOriginalColor(color);
    }
}

void ColorPicker::notify()
{
    emit colorChanged(m_color);
}

void ColorPicker::syncControls(QWidget* except)
{
    m_updating = true;

    const QColor color = m_color;

    m_plane->setAxis(currentAxis());
    m_ramp->setAxis(currentAxis());
    m_plane->setHsv(m_hue, m_sat, m_val);
    m_ramp->setHsv(m_hue, m_sat, m_val);
    m_compare->setCurrentColor(color);

    const auto assign = [except](QSpinBox* box, int value) {
        if (box != except) {
            box->setValue(value);
        }
    };

    assign(m_spinH, m_hue);
    assign(m_spinS, int(std::lround(m_sat * 100.0 / 255.0)));
    assign(m_spinB, int(std::lround(m_val * 100.0 / 255.0)));

    assign(m_spinR, color.red());
    assign(m_spinG, color.green());
    assign(m_spinBlue, color.blue());

    double l = 0.0;
    double a = 0.0;
    double b = 0.0;
    rgbToLab(color, &l, &a, &b);
    assign(m_spinL, int(std::lround(l)));
    assign(m_spinLabA, int(std::lround(a)));
    assign(m_spinLabB, int(std::lround(b)));

    const CmykPercent cmyk = rgbToCmyk(color);
    assign(m_spinC, int(std::lround(cmyk.c)));
    assign(m_spinM, int(std::lround(cmyk.m)));
    assign(m_spinY, int(std::lround(cmyk.y)));
    assign(m_spinK, int(std::lround(cmyk.k)));

    if (m_hex != except) {
        m_hex->setText(color.name().mid(1).toUpper());
    }

    updateAlerts();
    m_updating = false;
}

void ColorPicker::onAxisChanged()
{
    if (m_updating) {
        return;
    }
    syncControls();
}

void ColorPicker::onPlanePicked(int hue, int sat, int val)
{
    if (m_updating) {
        return;
    }
    setHsv(hue, sat, val);
    syncControls();
    notify();
}

void ColorPicker::onHsbFieldsEdited()
{
    if (m_updating) {
        return;
    }
    const int hue = m_spinH->value() % 360;
    setHsv(hue, int(std::lround(m_spinS->value() * 255.0 / 100.0)),
           int(std::lround(m_spinB->value() * 255.0 / 100.0)));
    syncControls(qobject_cast<QWidget*>(sender()));
    notify();
}

void ColorPicker::onRgbFieldsEdited()
{
    if (m_updating) {
        return;
    }
    QColor c(m_spinR->value(), m_spinG->value(), m_spinBlue->value());
    if (m_webOnly->isChecked()) {
        c = snapToWebColor(c);
    }
    m_color = c.toRgb();
    toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    syncControls(qobject_cast<QWidget*>(sender()));
    notify();
}

void ColorPicker::onLabFieldsEdited()
{
    if (m_updating) {
        return;
    }
    QColor c = labToRgb(m_spinL->value(), m_spinLabA->value(), m_spinLabB->value());
    if (m_webOnly->isChecked()) {
        c = snapToWebColor(c);
    }
    m_color = c.toRgb();
    toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    syncControls(qobject_cast<QWidget*>(sender()));
    notify();
}

bool ColorPicker::applyHex(const QString& text, bool normalize)
{
    QColor color;
    if (!parseHexColor(text, &color)) {
        return false;
    }
    if (m_webOnly->isChecked()) {
        color = snapToWebColor(color);
    }
    m_color = color.toRgb();
    toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    syncControls(m_hex);
    if (normalize) {
        const QString canonical = m_color.name().mid(1).toUpper();
        if (m_hex->text() != canonical) {
            m_updating = true;
            m_hex->setText(canonical);
            m_updating = false;
        }
    }
    notify();
    return true;
}

void ColorPicker::onCmykFieldsEdited()
{
    if (m_updating) {
        return;
    }
    const uint32_t rgb = pictura::picker_rgb(m_spinC->value(), m_spinM->value(),
                                             m_spinY->value(), m_spinK->value());
    if (rgb == 0xFFFFFFFFu) {
        return;
    }
    m_color = QColor::fromRgb(QRgb(rgb));
    toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    syncControls(qobject_cast<QWidget*>(sender()));
    notify();
}

void ColorPicker::updateAlerts()
{
    if (!m_webAlert || !m_gamutAlert) {
        return;
    }
    m_webAlert->setVisible(!m_webOnly->isChecked() && !isWebColor(m_color));
    m_gamutAlert->setVisible(
        !pictura::picker_in_gamut(m_color.red(), m_color.green(), m_color.blue()));
}

void ColorPicker::onHexEdited()
{
    if (m_updating) {
        return;
    }
    // Three-digit shorthand waits for the commit: typing "123" towards
    // "123456" must not turn into #112233 halfway through.
    if (hexDigits(m_hex->text()).size() == 6) {
        applyHex(m_hex->text(), false);
    }
}

void ColorPicker::onHexCommitted()
{
    if (m_updating) {
        return;
    }
    if (applyHex(m_hex->text(), true)) {
        return;
    }
    const QString canonical = m_color.name().mid(1).toUpper();
    if (m_hex->text() != canonical) {
        m_updating = true;
        m_hex->setText(canonical);
        m_updating = false;
    }
}

void ColorPicker::onWebColorsToggled(bool on)
{
    m_plane->setWebColorsOnly(on);
    m_ramp->setWebColorsOnly(on);
    if (on) {
        m_color = snapToWebColor(m_color).toRgb();
        toHsvPreservingHue(m_color, m_hue, m_sat, m_val);
    }
    syncControls();
    notify();
}

void ColorPicker::revertToOriginal()
{
    setColor(m_original);
    notify();
}

// ===========================================================================
// ColorPickerDialog
// ===========================================================================

ColorPickerDialog::ColorPickerDialog(const QColor& initial, QWidget* parent, const QString& title,
                                     SwatchGrid* swatches)
    : QDialog(parent)
    , m_swatches(swatches)
{
    setWindowTitle(title.isEmpty() ? tr("Color Picker")
                                   : tr("Color Picker (%1)").arg(title));
    setModal(true);

    auto* root = new QHBoxLayout(this);
    root->setContentsMargins(12, 12, 12, 12);
    root->setSpacing(10);

    m_picker = new ColorPicker(this);
    root->addWidget(m_picker);

    auto* buttons = new QVBoxLayout();
    buttons->setSpacing(5);

    auto* ok = new QPushButton(tr("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    auto* addSwatch = new QPushButton(tr("Add to Swatches"), this);
    addSwatch->setObjectName(QStringLiteral("colorPickerAddToSwatches"));
    auto* libraries = new QPushButton(tr("Color Libraries"), this);
    libraries->setObjectName(QStringLiteral("colorPickerLibraries"));

    if (m_swatches) {
        connect(addSwatch, &QPushButton::clicked, this, &ColorPickerDialog::addToSwatches);
    } else {
        addSwatch->setEnabled(false);
        addSwatch->setToolTip(tr("The Swatches panel is not available"));
    }
    // ponytail: Color Libraries need the #489 preset codecs and PANTONE
    // licensing; re-enable with the book loader.
    libraries->setEnabled(false);
    libraries->setToolTip(tr("Spot color libraries are not implemented yet"));

    for (QPushButton* b : {ok, cancel, addSwatch, libraries}) {
        b->setMinimumWidth(124);
        buttons->addWidget(b);
    }
    buttons->addStretch(1);
    root->addLayout(buttons);

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);

    const QColor start = initial.isValid() ? initial : QColor(Qt::black);
    m_picker->setColor(start);
    m_picker->setOriginalColor(start);
}

ColorPickerDialog::~ColorPickerDialog()
{
    if (m_tools) {
        m_tools->setCanvasSampler({});
    }
}

QColor ColorPickerDialog::selectedColor() const
{
    return m_picker ? m_picker->color() : QColor();
}

void ColorPickerDialog::setDefaultSwatchGrid(SwatchGrid* grid)
{
    g_swatchGrid = grid;
}

SwatchGrid* ColorPickerDialog::defaultSwatchGrid()
{
    return g_swatchGrid;
}

void ColorPickerDialog::addToSwatches()
{
    if (!m_swatches) {
        return;
    }
    const QColor color = m_picker->color();
    const QString suggested = m_swatches->nextSwatchName();
    QString name;
    if (!promptForSwatchName(this, color, suggested, &name)) {
        return;
    }
    m_swatches->addSwatch({color, name.isEmpty() ? suggested : name});
}

void ColorPickerDialog::sampleCanvas(ToolController* tools,
                                     std::function<QColor(const QPoint&)> colorAt)
{
    if (!tools || !colorAt) {
        return;
    }
    m_tools = tools;
    tools->setCanvasSampler([this, colorAt = std::move(colorAt)](const QPointF& imagePos) {
        const QColor color = colorAt(imagePos.toPoint());
        if (color.isValid()) {
            m_picker->setColor(color);
        }
    });
    connect(this, &QDialog::finished, this, [this]() {
        if (m_tools) {
            m_tools->setCanvasSampler({});
            m_tools = nullptr;
        }
    });
}

void ColorPickerDialog::setPickerPreference(int choice)
{
    g_pickerChoice = choice == 1 ? 1 : 0;
}

int ColorPickerDialog::pickerPreference()
{
    return g_pickerChoice;
}

QColor ColorPickerDialog::getColor(const QColor& initial, QWidget* parent, const QString& title,
                                   ToolController* tools,
                                   std::function<QColor(const QPoint&)> colorAt)
{
    if (g_pickerChoice == 1) {
        return QColorDialog::getColor(initial, parent, title);
    }
    ColorPickerDialog dialog(initial, parent, title, defaultSwatchGrid());
    dialog.sampleCanvas(tools, std::move(colorAt));
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return {};
    }
    return dialog.selectedColor();
}

} // namespace pictura

#include "moc_color_picker_dialog.cpp"
