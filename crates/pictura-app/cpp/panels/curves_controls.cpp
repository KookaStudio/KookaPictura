#include "curves_controls.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "color_models.h"
#include "curve_widget.h"
#include "icons.h"
#include "levels_dialog.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtCore/QtMath>
#include <QtGui/QPalette>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cstdint>
#include <utility>

namespace pictura {
namespace {

std::array<int, 5> codes(const QColor& color, const QString& mode)
{
    const int r = color.red(), g = color.green(), b = color.blue();
    if (mode == QLatin1String("cmyk")) {
        const int top = std::max({r, g, b});
        const auto ink = [top](int v) { return top ? qRound(255.0 * (top - v) / top) : 0; };
        return {255 - qGray(r, g, b), ink(r), ink(g), ink(b), 255 - top};
    }
    if (mode == QLatin1String("lab")) {
        double l, a, b;
        rgbToLab(color, &l, &a, &b);
        const int light = qBound(0, qRound(l * 2.55), 255);
        return {light, light, qBound(0, qRound(a + 128), 255), qBound(0, qRound(b + 128), 255), 0};
    }
    return {qGray(r, g, b), r, g, b, 0};
}

QToolButton* tool(QWidget* parent, const QString& label, const QString& name, bool checkable = false)
{
    auto* button = new QToolButton(parent);
    button->setText(label);
    button->setObjectName(name);
    button->setCheckable(checkable);
    return button;
}

} // namespace

CurveHistograms curvesHistograms(PictureView* view, const QString& mode)
{
    CurveHistograms result{};
    if (!view) {
        return result;
    }
    const auto levels = document_api::pyramidSizes(*view);
    for (int level = 0; level < levels.size(); ++level) {
        const auto size = levels[level];
        if (size.width > 1024 || size.height > 1024) {
            continue;
        }
        const QImage image =
            view->display_image(level, 0, 0, size.width, size.height).convertToFormat(QImage::Format_ARGB32);
        for (int y = 0; y < image.height(); ++y) {
            const auto* row = reinterpret_cast<const QRgb*>(image.constScanLine(y));
            for (int x = 0; x < image.width(); ++x) {
                if (qAlpha(row[x]) != 0) {
                    const auto values = codes(QColor::fromRgb(row[x]), mode);
                    for (size_t c = 0; c < values.size(); ++c) {
                        ++result[c][values[c]];
                    }
                }
            }
        }
        break;
    }
    return result;
}

QByteArray curvesGrayPointBlock(const QByteArray& block, const QColor& color)
{
    const auto values = codes(color, QStringLiteral("rgb"));
    const int gray = values[0];
    QByteArray edited = block;
    for (int channel = 0; channel < 4; ++channel) {
        const int input = values[channel];
        const QString points = channel == 0 ? QStringLiteral("0,0 255,255")
            : input == 0 ? QStringLiteral("0,%1 255,255").arg(gray)
            : input == 255 ? QStringLiteral("0,0 255,%1").arg(gray)
            : QStringLiteral("0,0 %1,%2 255,255").arg(input).arg(gray);
        const auto next = image_adjustment_set_curve(
            {reinterpret_cast<const std::uint8_t*>(edited.constData()), std::size_t(edited.size())},
            channel, points);
        if (next.size() == 0) {
            return {};
        }
        edited = QByteArray(reinterpret_cast<const char*>(next.data()), qsizetype(next.size()));
    }
    return edited;
}

bool curveAutoEndpoints(const CurveHistogram& bins, int& low, int& high)
{
    long long total = 0;
    for (int count : bins) {
        total += count;
    }
    if (!total) {
        return false;
    }
    const long long clip = total / 1000;
    low = 0;
    for (long long seen = 0; low < 255 && (seen += bins[low]) <= clip;) {
        ++low;
    }
    high = 255;
    for (long long seen = 0; high > 0 && (seen += bins[high]) <= clip;) {
        --high;
    }
    return high - low >= 2;
}

CurvesControls::CurvesControls(bool dialog, QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    channelRow_ = new QHBoxLayout;
    channel_ = new QComboBox(this);
    channel_->setObjectName(dialog ? QStringLiteral("curvesChannel") : QStringLiteral("adjustmentCurveChannel"));
    channel_->setAccessibleName(tr("Channel"));
    channelRow_->addWidget(channel_, 1);
    layout->addLayout(channelRow_);

    pencil_ = tool(this, tr("Pencil"), QStringLiteral("curvesPencil"), true);
    smooth_ = tool(this, tr("Smooth"), QStringLiteral("curvesSmooth"));
    target_ = tool(this, tr("On-image"), QStringLiteral("curvesTarget"), true);
    if (dialog) {
        auto* graphTools = new QHBoxLayout;
        for (auto* button : {pencil_, smooth_, target_}) {
            graphTools->addWidget(button);
        }
        layout->addLayout(graphTools);
    }

    auto* plotColumn = layout;
    QVBoxLayout* rail = nullptr;
    if (!dialog) {
        auto* editor = new QHBoxLayout;
        editor->setSpacing(6);
        rail = new QVBoxLayout;
        rail->setSpacing(2);
        plotColumn = new QVBoxLayout;
        plotColumn->setSpacing(3);
        editor->addLayout(rail);
        editor->addLayout(plotColumn, 1);
        layout->addLayout(editor);
    }

    graph_ = new CurveWidget(this);
    graph_->setObjectName(dialog ? QStringLiteral("curve") : QStringLiteral("adjustmentCurve"));
    graph_->enableCurvesControls();
    graph_->setShowRamps(true);
    if (dialog) {
        graph_->setFixedSize(graph_->sizeHint() + QSize(32, 32));
        plotColumn->addWidget(graph_);
        auto* endpoints = new QGridLayout;
        black_ = new QSlider(Qt::Horizontal, this);
        white_ = new QSlider(Qt::Horizontal, this);
        black_->setObjectName(QStringLiteral("curvesBlackSlider"));
        white_->setObjectName(QStringLiteral("curvesWhiteSlider"));
        for (auto* slider : {black_, white_}) {
            slider->setRange(0, 255);
            connect(slider, &QSlider::sliderPressed, this, &CurvesControls::gestureStarted);
            connect(slider, &QSlider::sliderReleased, this, &CurvesControls::gestureEnded);
        }
        endpoints->addWidget(new QLabel(tr("Black Input"), this), 0, 0);
        endpoints->addWidget(black_, 0, 1);
        endpoints->addWidget(new QLabel(tr("White Input"), this), 1, 0);
        endpoints->addWidget(white_, 1, 1);
        plotColumn->addLayout(endpoints);
        connect(black_, &QSlider::valueChanged, this, [this](int value) {
            graph_->setEndpoint(false, value, false);
        });
        connect(white_, &QSlider::valueChanged, this, [this](int value) {
            graph_->setEndpoint(true, value, false);
        });
    } else {
        graph_->useSquareLayout();
        plotColumn->addWidget(graph_, 0, Qt::AlignTop);
        endpoints_ = new TriangleSlider(2, this);
        endpoints_->setObjectName(QStringLiteral("curvesEndpoints"));
        endpoints_->setAccessibleName(tr("Black and white input endpoints"));
        endpoints_->setToolTip(tr("Drag an endpoint, or select it with Up/Down and adjust with Left/Right"));
        endpoints_->setFocusPolicy(Qt::StrongFocus);
        const auto insets = graph_->plotInsets();
        endpoints_->setTrackInsets(insets.left(), insets.right());
        endpoints_->setColor(0, Qt::black);
        endpoints_->setColor(1, Qt::white);
        plotColumn->addWidget(endpoints_);
        connect(endpoints_, &TriangleSlider::dragStarted, this, &CurvesControls::gestureStarted);
        connect(endpoints_, &TriangleSlider::dragEnded, this, &CurvesControls::gestureEnded);
        connect(endpoints_, &TriangleSlider::valueChanged, this, [this](int handle, int value) {
            graph_->setEndpoint(handle == 1, value, false);
        });
    }

    auto* points = new QHBoxLayout;
    input_ = new QSpinBox(this);
    output_ = new QSpinBox(this);
    input_->setObjectName(dialog ? QStringLiteral("curvesInput") : QStringLiteral("adjustmentCurveInput"));
    output_->setObjectName(dialog ? QStringLiteral("curvesOutput") : QStringLiteral("adjustmentCurveOutput"));
    for (auto* field : {input_, output_}) {
        field->setKeyboardTracking(false);
        if (dialog) {
            field->setMinimumWidth(68);
        }
        connect(field, &QSpinBox::valueChanged, this, [this, field] {
            const auto points = graph_->points();
            const int selected = graph_->selected();
            if (selected < 0 || selected >= points.size()) {
                return;
            }
            const QPointF point = points[selected];
            const int x = field == input_ ? storedCode(input_->value()) : qRound(point.x() * 255);
            const int y = field == output_ ? storedCode(output_->value()) : qRound(point.y() * 255);
            graph_->moveSelected(x, y);
            emit gestureEnded();
        });
    }
    const auto addField = [&](const QString& text, QSpinBox* field) {
        auto* label = new QLabel(text, this);
        label->setBuddy(field);
        field->setAccessibleName(text);
        if (dialog) {
            points->addWidget(label);
            points->addWidget(field);
        } else {
            auto* column = new QVBoxLayout;
            column->setSpacing(2);
            column->addWidget(label);
            column->addWidget(field);
            points->addLayout(column, 1);
        }
    };
    addField(tr("Input"), input_);
    addField(tr("Output"), output_);
    plotColumn->addLayout(points);
    if (!dialog) {
        plotColumn->addStretch(1);
    }

    blackPick_ = tool(this, tr("Black"), QStringLiteral("curvesBlackPoint"), true);
    grayPick_ = tool(this, tr("Gray"), QStringLiteral("curvesGrayPoint"), true);
    whitePick_ = tool(this, tr("White"), QStringLiteral("curvesWhitePoint"), true);
    if (dialog) {
        auto* pipettes = new QHBoxLayout;
        for (auto* button : {blackPick_, grayPick_, whitePick_}) {
            pipettes->addWidget(button);
        }
        layout->addLayout(pipettes);
    } else {
        point_ = tool(this, tr("Edit points"), QStringLiteral("curvesPointMode"), true);
        clipping_ = tool(this, tr("Show clipping"), QStringLiteral("curvesClipping"), true);
        const auto addTool = [&](QToolButton* button, const QString& iconId, const QString& label) {
            button->setIcon(icon(iconId));
            button->setIconSize(QSize(15, 15));
            button->setToolButtonStyle(Qt::ToolButtonIconOnly);
            button->setToolTip(label);
            button->setAccessibleName(label);
            rail->addWidget(button);
        };
        addTool(target_, QStringLiteral("curves.target"), tr("Adjust a tone on the image"));
        addTool(blackPick_, QStringLiteral("curves.blackPoint"), tr("Sample the black point"));
        addTool(grayPick_, QStringLiteral("curves.grayPoint"), tr("Sample the gray point"));
        addTool(whitePick_, QStringLiteral("curves.whitePoint"), tr("Sample the white point"));
        addTool(point_, QStringLiteral("adjustments.curves"), tr("Edit curve points"));
        addTool(pencil_, QStringLiteral("tool.pencil"), tr("Draw a freehand curve"));
        addTool(smooth_, QStringLiteral("curves.smooth"), tr("Smooth the freehand curve"));
        addTool(clipping_, QStringLiteral("curves.clipping"), tr("Show clipping"));
        rail->addStretch(1);
        connect(point_, &QToolButton::clicked, this, [this] {
            disarm();
            pencil_->setChecked(false);
            refreshFields();
        });
        connect(clipping_, &QToolButton::toggled, this, &CurvesControls::clippingPreviewChanged);
    }

    if (dialog) {
        auto* options = tool(this, tr("Display Options…"), QStringLiteral("curvesDisplayToggle"));
        layout->addWidget(options, 0, Qt::AlignLeft);
        connect(options, &QToolButton::clicked, this, &CurvesControls::openDisplayOptions);
    }
    connect(channel_, &QComboBox::currentIndexChanged, this, [this] {
        disarm();
        emit gestureEnded();
        reload();
    });
    connect(graph_, &CurveWidget::gridToggled, this, [this] {
        displayOptions_.detailedGrid = !displayOptions_.detailedGrid;
        refreshGraph();
    });
    connect(graph_, &CurveWidget::selectionChanged, this, &CurvesControls::refreshFields);
    connect(graph_, &CurveWidget::curveChanged, this, [this] {
        refreshFields();
        emit curveChanged(channel(), graph_->pointsText());
    });
    connect(graph_, &CurveWidget::gestureStarted, this, &CurvesControls::gestureStarted);
    connect(graph_, &CurveWidget::gestureEnded, this, &CurvesControls::gestureEnded);
    connect(pencil_, &QToolButton::toggled, this, [this](bool enabled) {
        disarm();
        if (graph_->setPencilEnabled(enabled)) {
            emit gestureEnded();
        }
        refreshFields();
    });
    connect(smooth_, &QToolButton::clicked, this, [this] {
        graph_->smoothPencil();
        emit gestureEnded();
    });
    connect(target_, &QToolButton::clicked, this, [this] { arm(Pick::Target); });
    connect(blackPick_, &QToolButton::clicked, this, [this] { arm(Pick::Black); });
    connect(grayPick_, &QToolButton::clicked, this, [this] { arm(Pick::Gray); });
    connect(whitePick_, &QToolButton::clicked, this, [this] { arm(Pick::White); });
    setContext(QStringLiteral("rgb"), {});
}

CurvesControls::~CurvesControls()
{
    targetGesture_ = false;
    disarm();
}

void CurvesControls::setContext(const QString& mode, const CurveHistograms& histograms)
{
    histograms_ = histograms;
    if (mode_ != mode) {
        disarm();
        mode_ = mode;
        const QSignalBlocker block(channel_);
        channel_->clear();
        if (mode == QLatin1String("cmyk")) {
            channel_->addItems({tr("CMYK"), tr("Cyan"), tr("Magenta"), tr("Yellow"), tr("Black")});
        } else if (mode == QLatin1String("lab")) {
            channel_->addItem(tr("Lightness"), 0);
            channel_->addItem(tr("a"), 2);
            channel_->addItem(tr("b"), 3);
        } else if (mode == QLatin1String("grayscale")) {
            channel_->addItem(tr("Gray"));
        } else {
            channel_->addItems({tr("RGB"), tr("Red"), tr("Green"), tr("Blue")});
        }
        displayOptions_.ink = mode == QLatin1String("cmyk");
        if (auto* options = findChild<QDialog*>(QStringLiteral("curvesDisplayOptions"),
                                               Qt::FindDirectChildrenOnly)) {
            options->reject();
        }
    }
    refreshGraph();
}

int CurvesControls::channel() const
{
    return channel_->currentData().isValid() ? channel_->currentData().toInt() : std::max(0, channel_->currentIndex());
}

bool CurvesControls::autoAvailable() const
{
    return mode_ == QLatin1String("rgb") || mode_ == QLatin1String("grayscale");
}

bool CurvesControls::reversed() const
{
    return displayOptions_.ink != (mode_ == QLatin1String("cmyk"));
}

int CurvesControls::displayCode(int code) const
{
    if (reversed()) {
        code = 255 - code;
    }
    if (displayOptions_.ink || (mode_ == QLatin1String("lab") && channel() == 0)) {
        return qRound(code / 2.55);
    }
    return mode_ == QLatin1String("lab") ? code - 128 : code;
}

int CurvesControls::storedCode(int display) const
{
    if (displayOptions_.ink || (mode_ == QLatin1String("lab") && channel() == 0)) {
        display = qRound(display * 2.55);
    } else if (mode_ == QLatin1String("lab")) {
        display += 128;
    }
    return reversed() ? 255 - display : display;
}

void CurvesControls::refreshFields()
{
    const auto points = graph_->points();
    const int selected = graph_->selected();
    const bool point = selected >= 0 && selected < points.size() && !pencil_->isChecked();
    for (auto* field : {input_, output_}) {
        const QSignalBlocker block(field);
        const int zero = displayCode(0);
        const int full = displayCode(255);
        field->setRange(std::min(zero, full), std::max(zero, full));
        field->setSuffix(displayOptions_.ink ? QStringLiteral(" %") : QString());
        field->setEnabled(point);
        const QPointF current = point ? points[selected] : QPointF();
        field->setValue(displayCode(qRound((field == input_ ? current.x() : current.y()) * 255)));
    }
    const bool editable = points.size() >= 2 && !pencil_->isChecked();
    if (endpoints_) {
        endpoints_->setEnabled(editable);
        endpoints_->setInvertedAppearance(reversed());
        if (editable) {
            const int lo = qRound(255 * points.first().x());
            const int hi = qRound(255 * points.last().x());
            const int next = qRound(255 * points[1].x());
            const int previous = qRound(255 * points[points.size() - 2].x());
            endpoints_->setRange(0, 0, next - 1);
            endpoints_->setRange(1, previous + 1, 255);
            endpoints_->setValue(0, lo);
            endpoints_->setValue(1, hi);
        }
    }
    for (auto* slider : {black_, white_}) {
        if (!slider) {
            continue;
        }
        const QSignalBlocker block(slider);
        slider->setEnabled(editable);
        slider->setInvertedAppearance(reversed());
        slider->setValue(editable ? qRound(255 * (slider == black_ ? points.first().x() : points.last().x())) : 0);
    }
    if (point_) {
        point_->setChecked(!pencil_->isChecked());
        clipping_->setEnabled(autoAvailable());
    }
    if (auto_) {
        auto_->setEnabled(autoAvailable());
    }
    smooth_->setEnabled(graph_->hasDenseCurve());
    target_->setEnabled(tools_ && editable);
    const bool endpoints = tools_ && editable && !(mode_ == QLatin1String("lab") && channel() != 0);
    blackPick_->setEnabled(endpoints);
    whitePick_->setEnabled(endpoints);
    grayPick_->setEnabled(endpoints && mode_ == QLatin1String("rgb") && channel() == 0);
}

void CurvesControls::refreshGraph()
{
    graph_->setGraphOptions(displayOptions_.detailedGrid, displayOptions_.intersectionLines, reversed());
    graph_->setShowHistogram(displayOptions_.histogram);
    graph_->setShowBaseline(displayOptions_.baseline);
    graph_->setBins(histograms_[channel()]);
    QVector<QPair<QColor, QString>> curves;
    if (source_ && channel() == 0 && displayOptions_.channelOverlays) {
        const QList<QColor> colors = mode_ == QLatin1String("cmyk")
            ? QList<QColor>{Qt::cyan, Qt::magenta, Qt::yellow, palette().color(QPalette::Mid)}
            : QList<QColor>{Qt::red, Qt::green, Qt::blue};
        if (mode_ == QLatin1String("rgb") || mode_ == QLatin1String("cmyk")) {
            for (int i = 0; i < colors.size(); ++i) {
                curves.append({colors[i], source_(i + 1)});
            }
        }
    }
    graph_->setOverlays(curves);
    refreshFields();
}

void CurvesControls::reload()
{
    if (source_ && channel_->currentIndex() >= 0) {
        const QString points = source_(channel());
        if (graph_->pointsText() != points) {
            graph_->setPointsText(points);
        }
        const QSignalBlocker block(pencil_);
        pencil_->setChecked(graph_->hasDenseCurve());
    }
    refreshGraph();
}

void CurvesControls::applyAuto()
{
    if (!autoAvailable() || !source_) {
        return;
    }
    int low, high;
    if (!curveAutoEndpoints(histograms_[channel()], low, high)) {
        return;
    }
    const QString points = QStringLiteral("%1,0 %2,255").arg(low).arg(high);
    if (points != source_(channel())) {
        emit curveChanged(channel(), points);
        emit gestureEnded();
        reload();
    }
}

void CurvesControls::setAutoButton(QPushButton* button)
{
    auto_ = button;
    if (auto_) {
        channelRow_->addWidget(auto_);
        auto_->setEnabled(autoAvailable());
    }
}

void CurvesControls::setClippingPreview(bool enabled)
{
    if (clipping_) {
        const QSignalBlocker block(clipping_);
        clipping_->setChecked(enabled && clipping_->isEnabled());
    }
}

void CurvesControls::openDisplayOptions()
{
    const auto name = QStringLiteral("curvesDisplayOptions");
    if (auto* existing = findChild<QDialog*>(name, Qt::FindDirectChildrenOnly);
        existing && existing->isVisible()) {
        existing->raise();
        existing->activateWindow();
        return;
    }

    auto* options = new QDialog(this);
    options->setObjectName(name);
    options->setWindowTitle(tr("Curves Display Options"));
    options->setAttribute(Qt::WA_DeleteOnClose);
    auto* layout = new QVBoxLayout(options);

    auto* amount = new QGroupBox(tr("Show Amount Of"), options);
    auto* amountRow = new QHBoxLayout(amount);
    auto* light = new QRadioButton(tr("Light (0–255)"), amount);
    auto* ink = new QRadioButton(tr("Pigment/Ink %"), amount);
    light->setObjectName(QStringLiteral("curvesLightUnits"));
    ink->setObjectName(QStringLiteral("curvesInkUnits"));
    light->setChecked(!displayOptions_.ink);
    ink->setChecked(displayOptions_.ink);
    amount->setEnabled(mode_ != QLatin1String("lab"));
    amountRow->addWidget(light);
    amountRow->addWidget(ink);
    layout->addWidget(amount);

    auto* grid = new QGroupBox(tr("Grid"), options);
    auto* gridRow = new QHBoxLayout(grid);
    auto* standard = new QRadioButton(tr("Standard"), grid);
    auto* detailed = new QRadioButton(tr("Detailed"), grid);
    standard->setObjectName(QStringLiteral("curvesStandardGrid"));
    detailed->setObjectName(QStringLiteral("curvesDetailedGrid"));
    standard->setChecked(!displayOptions_.detailedGrid);
    detailed->setChecked(displayOptions_.detailedGrid);
    gridRow->addWidget(standard);
    gridRow->addWidget(detailed);
    layout->addWidget(grid);

    auto* show = new QGroupBox(tr("Show"), options);
    auto* showLayout = new QVBoxLayout(show);
    const auto addCheckbox = [&](const QString& label, const QString& objectName, bool checked) {
        auto* box = new QCheckBox(label, show);
        box->setObjectName(objectName);
        box->setChecked(checked);
        showLayout->addWidget(box);
        return box;
    };
    auto* overlays = addCheckbox(tr("Channel Overlays"), QStringLiteral("curvesOverlays"),
                                 displayOptions_.channelOverlays);
    auto* histogram = addCheckbox(tr("Histogram"), QStringLiteral("curvesHistogram"),
                                  displayOptions_.histogram);
    auto* baseline = addCheckbox(tr("Baseline"), QStringLiteral("curvesBaseline"),
                                 displayOptions_.baseline);
    auto* guides = addCheckbox(tr("Intersection Line"), QStringLiteral("curvesIntersectionLine"),
                               displayOptions_.intersectionLines);
    layout->addWidget(show);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, options);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, options, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, options, &QDialog::reject);
    connect(options, &QDialog::accepted, this, [this, ink, detailed, overlays, histogram, baseline, guides] {
        displayOptions_ = {ink->isChecked(), detailed->isChecked(), overlays->isChecked(),
                           histogram->isChecked(), baseline->isChecked(), guides->isChecked()};
        refreshGraph();
    });
    options->open();
}

void CurvesControls::setCanvas(ToolController* tools, CanvasSample sample)
{
    disarm();
    tools_ = tools;
    sample_ = std::move(sample);
    refreshFields();
}

void CurvesControls::disarm()
{
    if (pick_ != Pick::None && tools_) {
        tools_->setCanvasBrush({});
    }
    pick_ = Pick::None;
    targetInput_ = -1;
    for (auto* button : {target_, blackPick_, grayPick_, whitePick_}) {
        button->setChecked(false);
    }
    if (targetGesture_) {
        targetGesture_ = false;
        emit gestureEnded();
    }
}

void CurvesControls::setTargeting(bool enabled)
{
    if (enabled && pick_ == Pick::None && target_->isEnabled()) {
        arm(Pick::Target);
    } else if (!enabled && pick_ == Pick::Target) {
        disarm();
    }
}

void CurvesControls::arm(Pick pick)
{
    const bool wasArmed = pick_ == pick;
    disarm();
    if (wasArmed || !tools_ || !sample_ || tools_->canvasBrushActive()
        || tools_->canvasSamplerActive()) {
        return;
    }
    pick_ = pick;
    auto* button = pick == Pick::Target ? target_ : pick == Pick::Black ? blackPick_
        : pick == Pick::White ? whitePick_ : grayPick_;
    button->setChecked(true);
    tools_->setCanvasBrush([this](ToolController::CanvasBrushPhase phase, const QPointF& at,
                                  Qt::KeyboardModifiers) {
        using Phase = ToolController::CanvasBrushPhase;
        if (phase == Phase::Press) {
            const QColor color = sample_(QPoint(qFloor(at.x()), qFloor(at.y())));
            if (!color.isValid()) {
                return;
            }
            const int value = codes(color, mode_)[channel()];
            targetGesture_ = true;
            emit gestureStarted();
            if (pick_ == Pick::Target) {
                targetY_ = at.y();
                targetInput_ = graph_->selectTone(value) ? value : -1;
                targetOutput_ = graph_->outputAt(value);
            } else if (pick_ == Pick::Gray) {
                emit grayPointPicked(color);
            } else {
                const bool white = pick_ == Pick::White;
                const bool ink = mode_ == QLatin1String("cmyk");
                graph_->setEndpoint(ink ? !white : white, value);
            }
        } else if (phase == Phase::Drag && pick_ == Pick::Target && targetInput_ >= 0) {
            const int delta = qRound((targetY_ - at.y()) * (reversed() ? -1 : 1));
            graph_->moveSelected(targetInput_, targetOutput_ + delta);
        } else if (phase == Phase::Release) {
            targetInput_ = -1;
            if (targetGesture_) {
                targetGesture_ = false;
                emit gestureEnded();
            }
            if (pick_ != Pick::Target) {
                const Pick completed = pick_;
                QTimer::singleShot(0, this, [this, completed] {
                    if (pick_ == completed) {
                        disarm();
                    }
                });
            }
        }
    });
}

} // namespace pictura

#include "moc_curves_controls.cpp"
