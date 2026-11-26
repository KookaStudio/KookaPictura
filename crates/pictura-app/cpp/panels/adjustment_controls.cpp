#include "adjustment_controls.h"

#include "adjustment_dialog.h"

#include "color_picker_dialog.h"
#include "curve_widget.h"
#include "histogram_panel.h"
#include "jump_slider.h"
#include "levels_dialog.h"
#include "theme.h"

#include "api/filters_bridge.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <memory>

namespace pictura {

namespace {

QIcon swatch(const QColor& color)
{
    QPixmap pixmap(16, 16);
    pixmap.fill(color);
    QPainter p(&pixmap);
    p.setPen(QColor(0, 0, 0, 160));
    p.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    return QIcon(pixmap);
}

// A label and a control on one line.
QWidget* labelled(const QString& label, QWidget* control)
{
    auto* container = new QWidget();
    auto* row = new QHBoxLayout(container);
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(new QLabel(label + QLatin1Char(':'), container));
    control->setParent(container);
    row->addWidget(control, 1);
    return container;
}

} // namespace

AdjustmentControls::AdjustmentControls(QWidget* parent)
    : QWidget(parent)
{
    layout_ = new QVBoxLayout(this);
    layout_->setContentsMargins(0, 0, 0, 0);
    layout_->addStretch(1);
}

void AdjustmentControls::setPage(const QStringList& page, bool holdValues)
{
    // Rebuild only for another set of controls; otherwise reload the values,
    // except mid-gesture, when the controls lead.
    QString signature;
    for (const QString& line : page) {
        signature += line.section(QLatin1Char('\t'), 0, 1) + QLatin1Char('\n');
    }
    if (signature != signature_) {
        build(page);
        signature_ = signature;
    } else if (!holdValues) {
        load(page);
    }
    if (levelsRefresh_) levelsRefresh_();
}

void AdjustmentControls::refreshThresholdHistogram(const PictureView& view, const QString& path)
{
    if (!thresholdHistogram_) {
        return;
    }
    const auto counts = image_adjustment_threshold_histogram(view, path);
    QVector<quint32> bins;
    bins.reserve(qsizetype(counts.size()));
    for (auto count : counts) {
        bins.append(count);
    }
    thresholdHistogram_->setSeries(
        {{bins, Theme::tokenColor(QStringLiteral("graph-axis"))}}, false);
}

void AdjustmentControls::insert(QWidget* widget)
{
    layout_->insertWidget(layout_->count() - 1, widget);
}

void AdjustmentControls::clear()
{
    while (layout_->count() > 1) {
        QLayoutItem* item = layout_->takeAt(0);
        delete item->widget();
        delete item;
    }
    rows_.clear();
    groups_ = nullptr;
    curve_ = nullptr;
    thresholdHistogram_ = nullptr;
    curveChannel_ = nullptr;
    curveDisplay_ = nullptr;
    levelsRefresh_ = {};
}

void AdjustmentControls::build(const QStringList& page)
{
    clear();
    const bool threshold = page.contains(QStringLiteral("threshold-histogram"));
    loading_ = true;
    for (const QString& line : page) {
        const QStringList cells = line.split(QLatin1Char('\t'));
        const QString& kind = cells.at(0);
        if (kind == QLatin1String("title")) {
            title_ = cells.value(1);
        } else if (kind == QLatin1String("note")) {
            auto* note = new QLabel(cells.value(1));
            note->setWordWrap(true);
            note->setEnabled(false);
            insert(note);
        } else if (kind == QLatin1String("groups")) {
            groups_ = new QComboBox();
            groups_->setObjectName(QStringLiteral("adjustmentGroup"));
            groups_->addItems(cells.value(1).split(QLatin1Char('|')));
            insert(groups_);
            connect(groups_, &QComboBox::currentIndexChanged, this,
                    &AdjustmentControls::showGroup);
        } else if (kind == QLatin1String("curves")) {
            addCurves();
        } else if (kind == QLatin1String("slider")) {
            addSlider(cells, threshold);
        } else if (kind == QLatin1String("check")) {
            addCheck(cells);
        } else if (kind == QLatin1String("choice")) {
            addChoice(cells);
        } else if (kind == QLatin1String("color")) {
            addColor(cells);
        }
    }
    if (title_ == QLatin1String("Levels")) {
        addLevelsHandles();
    }
    showGroup(groups_ ? groups_->currentIndex() : 0);
    loading_ = false;
}

void AdjustmentControls::load(const QStringList& page)
{
    loading_ = true;
    for (const QString& line : page) {
        const QStringList cells = line.split(QLatin1Char('\t'));
        for (Row& row : rows_) {
            if (cells.size() > 4 && row.key == cells.at(1) && row.load) {
                row.load(cells.at(4).toDouble());
            }
        }
    }
    loadCurve();
    loading_ = false;
}

void AdjustmentControls::addSlider(const QStringList& cells, bool threshold)
{
    const QString key = cells.value(1);
    const int decimals = cells.value(8).toInt();
    const double factor = std::pow(10.0, decimals);
    auto* holder = new QWidget();
    auto* column = new QVBoxLayout(holder);
    column->setContentsMargins(0, 0, 0, 4);
    auto* spin = new QDoubleSpinBox();
    spin->setDecimals(decimals);
    spin->setRange(cells.value(6).toDouble(), cells.value(7).toDouble());
    spin->setKeyboardTracking(false);
    // CS6's value row, as every Adjustments dialog lays it out: the label,
    // then a fixed-width field at the right, over the slider.
    spin->setFixedWidth(AdjustmentDialog::kFieldWidth);
    auto* head = new QHBoxLayout;
    head->addWidget(new QLabel(cells.value(2) + QLatin1Char(':'), holder));
    head->addStretch(1);
    head->addWidget(spin);
    const QString unit = cells.value(9);
    if (!unit.isEmpty()) {
        auto* label = new QLabel(unit, holder);
        label->setObjectName(QStringLiteral("adjustmentUnit"));
        head->addWidget(label);
    }
    column->addLayout(head);
    if (threshold) {
        thresholdHistogram_ = new HistogramView(holder);
        thresholdHistogram_->setObjectName(QStringLiteral("thresholdHistogram"));
        thresholdHistogram_->setAccessibleName(tr("Luminance"));
        thresholdHistogram_->setMinimumHeight(64);
        column->addWidget(thresholdHistogram_);
    }
    auto* slider = new JumpSlider(Qt::Horizontal, holder);
    slider->setObjectName(QStringLiteral("adjustmentSlider.") + key);
    slider->setRange(qRound(spin->minimum() * factor), qRound(spin->maximum() * factor));
    column->addWidget(slider);
    insert(holder);

    const auto load = [slider, spin, factor](double v) {
        const QSignalBlocker a(slider);
        const QSignalBlocker b(spin);
        spin->setValue(v);
        slider->setValue(qRound(v * factor));
    };
    load(cells.value(4).toDouble());
    connect(slider, &QSlider::valueChanged, this, [this, key, spin, factor](int v) {
        const QSignalBlocker block(spin);
        spin->setValue(v / factor);
        if (!loading_) {
            emit valueChanged(key, v / factor);
        }
    });
    connect(slider, &QSlider::sliderReleased, this, &AdjustmentControls::gestureEnded);
    connect(spin, &QDoubleSpinBox::valueChanged, this, [this, key, slider, factor](double v) {
        const QSignalBlocker block(slider);
        slider->setValue(qRound(v * factor));
        if (!loading_) {
            emit valueChanged(key, v);
        }
    });
    rows_.push_back({key, cells.value(3).toInt(), holder, slider, load});
}

void AdjustmentControls::addCheck(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* box = new QCheckBox(cells.value(2));
    box->setObjectName(QStringLiteral("adjustmentCheck.") + key);
    const auto load = [box](double v) {
        const QSignalBlocker block(box);
        box->setChecked(v != 0.0);
    };
    load(cells.value(4).toDouble());
    connect(box, &QCheckBox::toggled, this, [this, key](bool on) {
        if (!loading_) {
            emit valueChanged(key, on ? 1.0 : 0.0);
            emit gestureEnded();
        }
    });
    insert(box);
    rows_.push_back({key, cells.value(3).toInt(), box, box, load});
}

void AdjustmentControls::addChoice(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* combo = new QComboBox();
    combo->setObjectName(QStringLiteral("adjustmentChoice.") + key);
    combo->addItems(cells.value(6).split(QLatin1Char('|')));
    QWidget* container = labelled(cells.value(2), combo);
    const auto load = [combo](double v) {
        const QSignalBlocker block(combo);
        combo->setCurrentIndex(int(v));
    };
    load(cells.value(4).toDouble());
    connect(combo, &QComboBox::currentIndexChanged, this, [this, key](int i) {
        if (!loading_) {
            emit valueChanged(key, i);
            emit gestureEnded();
        }
    });
    insert(container);
    rows_.push_back({key, cells.value(3).toInt(), container, combo, load});
}

void AdjustmentControls::addColor(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* button = new QToolButton();
    button->setObjectName(QStringLiteral("adjustmentColor.") + key);
    button->setAutoRaise(true);
    QWidget* container = labelled(cells.value(2), button);
    auto color = std::make_shared<QColor>();
    const auto load = [button, color](double v) {
        *color = QColor::fromRgb(QRgb(uint(v)) | 0xff000000u);
        button->setIcon(swatch(*color));
    };
    load(cells.value(4).toDouble());
    connect(button, &QToolButton::clicked, this, [this, key, color, load]() {
        const QColor picked = ColorPickerDialog::getColor(*color, this, QStringLiteral("Color"));
        if (picked.isValid()) {
            const double rgb = double(picked.rgb() & 0xffffffu);
            load(rgb);
            emit valueChanged(key, rgb);
            emit gestureEnded();
        }
    });
    insert(container);
    rows_.push_back({key, cells.value(3).toInt(), container, button, load});
}

void AdjustmentControls::addLevelsHandles()
{
    auto* holder = new QWidget(this);
    auto* layout = new QVBoxLayout(holder);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* pointRow = new QHBoxLayout;
    const QStringList pointNames{tr("Black point"), tr("Gray point"), tr("White point")};
    for (int point = 0; point < 3; ++point) {
        auto* button = new QToolButton(holder);
        button->setObjectName(QStringLiteral("propertiesLevelsPoint%1").arg(point));
        button->setText(pointNames[point]);
        button->setAccessibleName(pointNames[point]);
        pointRow->addWidget(button);
        connect(button, &QToolButton::clicked, this, [this, point] { emit levelsPickRequested(point); });
    }
    layout->addLayout(pointRow);
    auto* histogram = new LevelsStrip(false, 100, holder);
    histogram->setObjectName(QStringLiteral("propertiesLevelsHistogram"));
    layout->addWidget(histogram);
    auto* input = new TriangleSlider(3, holder);
    input->setObjectName(QStringLiteral("propertiesLevelsInput"));
    input->setColor(0, Qt::black);
    input->setColor(1, QColor(128, 128, 128));
    input->setColor(2, Qt::white);
    layout->addWidget(input);
    layout->addWidget(new LevelsStrip(true, 16, holder));
    auto* output = new TriangleSlider(2, holder);
    output->setObjectName(QStringLiteral("propertiesLevelsOutput"));
    output->setColor(0, Qt::black);
    output->setColor(1, Qt::white);
    layout->addWidget(output);
    layout_->insertWidget(1, holder);
    const auto slider = [this](const QString& key) {
        const QStringList prefixes{QString(), QStringLiteral("red."), QStringLiteral("green."), QStringLiteral("blue.")};
        return qobject_cast<QSlider*>(controlForTest(prefixes.value(groups_->currentIndex()) + key));
    };
    const auto update = [this, histogram, input, output, slider] {
        if (!groups_) return;
        auto* black = slider(QStringLiteral("inputBlack"));
        auto* white = slider(QStringLiteral("inputWhite"));
        auto* gamma = slider(QStringLiteral("gamma"));
        auto* outBlack = slider(QStringLiteral("outputBlack"));
        auto* outWhite = slider(QStringLiteral("outputWhite"));
        if (!black || !white || !gamma || !outBlack || !outWhite) return;
        const int lo = black->value();
        const int hi = white->value();
        input->setRange(0, 0, qMax(0, hi - 2));
        input->setRange(1, lo + 1, qMax(lo + 1, hi - 1));
        input->setRange(2, qMin(255, lo + 2), 255);
        input->setValue(0, lo);
        input->setValue(2, hi);
        input->setValue(1, lo + qRound(std::pow(0.5, gamma->value() / 100.0) * (hi - lo)));
        output->setValue(0, outBlack->value());
        output->setValue(1, outWhite->value());
        if (levelsHistogramSource_) histogram->setBins(levelsHistogramSource_(groups_->currentIndex()));
    };
    connect(groups_, &QComboBox::currentIndexChanged, holder, [this, update](int) { update(); emit levelsChannelChanged(); });
    for (const Row& row : rows_) {
        if (auto* value = qobject_cast<QSlider*>(row.control)) {
            connect(value, &QSlider::valueChanged, holder, [update](int) { update(); });
        }
    }
    connect(input, &TriangleSlider::valueChanged, holder, [slider](int index, int value) {
        auto* black = slider(QStringLiteral("inputBlack"));
        auto* white = slider(QStringLiteral("inputWhite"));
        auto* gamma = slider(QStringLiteral("gamma"));
        if (!black || !white || !gamma) return;
        if (index == 0) black->setValue(value);
        else if (index == 2) white->setValue(value);
        else {
            const double norm = std::clamp(double(value - black->value()) / qMax(1, white->value() - black->value()), 0.001, 0.999);
            gamma->setValue(qRound(100.0 * std::log(norm) / std::log(0.5)));
        }
    });
    connect(output, &TriangleSlider::valueChanged, holder, [slider](int index, int value) {
        if (auto* field = slider(index == 0 ? QStringLiteral("outputBlack") : QStringLiteral("outputWhite"))) field->setValue(value);
    });
    levelsRefresh_ = update;
    update();
}

void AdjustmentControls::addCurves()
{
    curveChannel_ = new QComboBox();
    curveChannel_->setObjectName(QStringLiteral("adjustmentCurveChannel"));
    curveChannel_->addItems({QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                             QStringLiteral("Blue")});
    insert(curveChannel_);
    curve_ = new CurveWidget();
    curve_->setObjectName(QStringLiteral("adjustmentCurve"));
    curve_->setMinimumSize(180, 180);
    curve_->setShowHistogram(true);
    curve_->setShowRamps(true);
    auto* tools = new QWidget(this);
    auto* toolRow = new QHBoxLayout(tools);
    toolRow->setContentsMargins(0, 0, 0, 0);
    const QStringList labels{tr("On-image"), tr("Black"), tr("Gray"), tr("White")};
    const QStringList names{QStringLiteral("curvesTarget"), QStringLiteral("curvesBlackPoint"),
                            QStringLiteral("curvesGrayPoint"), QStringLiteral("curvesWhitePoint")};
    for (int action = 0; action < labels.size(); ++action) {
        auto* button = new QToolButton(tools);
        button->setObjectName(names[action]);
        button->setText(labels[action]);
        button->setAccessibleName(labels[action]);
        toolRow->addWidget(button);
        connect(button, &QToolButton::clicked, this, [this, action] { emit curvesCanvasRequested(action); });
    }
    insert(tools);
    insert(curve_);
    auto* endpoints = new QWidget(this);
    auto* endpointLayout = new QHBoxLayout(endpoints);
    endpointLayout->setContentsMargins(0, 0, 0, 0);
    auto* black = new QSlider(Qt::Horizontal, endpoints);
    auto* white = new QSlider(Qt::Horizontal, endpoints);
    black->setObjectName(QStringLiteral("curvesBlackSlider"));
    white->setObjectName(QStringLiteral("curvesWhiteSlider"));
    for (auto* slider : {black, white}) {
        slider->setRange(0, 255);
        endpointLayout->addWidget(slider);
        connect(slider, &QSlider::sliderReleased, this, &AdjustmentControls::gestureEnded);
    }
    connect(black, &QSlider::valueChanged, this, [this](int value) { curve_->setEndpoint(false, value); });
    connect(white, &QSlider::valueChanged, this, [this](int value) { curve_->setEndpoint(true, value); });
    const auto refreshEndpoints = [this, black, white] {
        const auto points = curve_->points();
        if (points.size() < 2) return;
        const QSignalBlocker b(black), w(white);
        black->setValue(qRound(points.first().x() * 255));
        white->setValue(qRound(points.last().x() * 255));
    };
    connect(curve_, &CurveWidget::curveChanged, endpoints, refreshEndpoints);
    insert(endpoints);
    curveDisplay_ = new QWidget(this);
    curveDisplay_->setObjectName(QStringLiteral("curvesDisplayOptions"));
    auto* displayLayout = new QHBoxLayout(curveDisplay_);
    displayLayout->setContentsMargins(0, 0, 0, 0);
    auto* histogramOption = new QCheckBox(tr("Histogram"), curveDisplay_);
    auto* baselineOption = new QCheckBox(tr("Baseline"), curveDisplay_);
    histogramOption->setChecked(true);
    baselineOption->setChecked(true);
    displayLayout->addWidget(histogramOption);
    displayLayout->addWidget(baselineOption);
    connect(histogramOption, &QCheckBox::toggled, curve_, &CurveWidget::setShowHistogram);
    connect(baselineOption, &QCheckBox::toggled, curve_, &CurveWidget::setShowBaseline);
    insert(curveDisplay_);
    curveDisplay_->hide();
    auto* pointFields = new QWidget(this);
    auto* pointLayout = new QHBoxLayout(pointFields);
    pointLayout->setContentsMargins(0, 0, 0, 0);
    auto* input = new QSpinBox(pointFields);
    auto* output = new QSpinBox(pointFields);
    input->setObjectName(QStringLiteral("adjustmentCurveInput"));
    output->setObjectName(QStringLiteral("adjustmentCurveOutput"));
    input->setRange(0, 255);
    output->setRange(0, 255);
    input->setKeyboardTracking(false);
    output->setKeyboardTracking(false);
    pointLayout->addWidget(new QLabel(tr("Input"), pointFields));
    pointLayout->addWidget(input);
    pointLayout->addWidget(new QLabel(tr("Output"), pointFields));
    pointLayout->addWidget(output);
    insert(pointFields);
    const auto refreshPoint = [this, input, output] {
        const int selected = curve_->selected();
        const auto points = curve_->points();
        const bool valid = selected >= 0 && selected < points.size();
        input->setEnabled(valid);
        output->setEnabled(valid);
        if (valid) {
            const QSignalBlocker blockInput(input);
            const QSignalBlocker blockOutput(output);
            input->setValue(qRound(points[selected].x() * 255.0));
            output->setValue(qRound(points[selected].y() * 255.0));
        }
    };
    connect(curve_, &CurveWidget::selectionChanged, this, refreshPoint);
    connect(curve_, &CurveWidget::curveChanged, this, refreshPoint);
    connect(input, &QSpinBox::valueChanged, this, [this, input, output](int) {
        curve_->moveSelected(input->value(), output->value());
    });
    connect(output, &QSpinBox::valueChanged, this, [this, input, output](int) {
        curve_->moveSelected(input->value(), output->value());
    });
    refreshPoint();
    connect(curveChannel_, &QComboBox::currentIndexChanged, this, [this]() {
        emit gestureEnded();
        loading_ = true;
        loadCurve();
        if (levelsHistogramSource_) curve_->setBins(levelsHistogramSource_(curveChannel_->currentIndex()));
        loading_ = false;
        emit levelsChannelChanged();
    });
    connect(curve_, &CurveWidget::curveChanged, this, [this]() {
        if (!loading_) {
            emit curveChanged(curveChannel_->currentIndex(), curve_->pointsText());
        }
    });
    loadCurve();
    if (levelsHistogramSource_) curve_->setBins(levelsHistogramSource_(curveChannel_->currentIndex()));
    refreshEndpoints();
}

void AdjustmentControls::setLevelsHistogramSource(std::function<std::array<int, 256>(int)> source)
{
    levelsHistogramSource_ = std::move(source);
    if (curve_ && curveChannel_ && levelsHistogramSource_) {
        curve_->setBins(levelsHistogramSource_(curveChannel_->currentIndex()));
    }
}

int AdjustmentControls::curveChannel() const
{
    return curveChannel_ ? curveChannel_->currentIndex() : 0;
}

void AdjustmentControls::toggleCurvesDisplay()
{
    if (curveDisplay_) curveDisplay_->setVisible(!curveDisplay_->isVisible());
}

void AdjustmentControls::loadCurve()
{
    if (!curve_ || !curveSource_) {
        return;
    }
    curve_->setPointsText(curveSource_(curveChannel_->currentIndex()));
}

void AdjustmentControls::showGroup(int group)
{
    for (const Row& row : rows_) {
        row.widget->setVisible(row.group < 0 || row.group == group);
    }
}

bool AdjustmentControls::levelsClippingParameters(int& channel, int& black, int& white) const
{
    if (title_ == QLatin1String("Curves") && curve_ && curveChannel_) {
        const auto points = curve_->points();
        if (points.size() < 2) return false;
        channel = curveChannel_->currentIndex();
        black = qRound(points.first().x() * 255);
        white = qRound(points.last().x() * 255);
        return true;
    }
    if (title_ != QLatin1String("Levels") || !groups_) return false;
    const QStringList prefixes{QString(), QStringLiteral("red."), QStringLiteral("green."), QStringLiteral("blue.")};
    channel = groups_->currentIndex();
    auto* low = qobject_cast<QSlider*>(controlForTest(prefixes.value(channel) + QStringLiteral("inputBlack")));
    auto* high = qobject_cast<QSlider*>(controlForTest(prefixes.value(channel) + QStringLiteral("inputWhite")));
    if (!low || !high) return false;
    black = low->value();
    white = high->value();
    return true;
}

QWidget* AdjustmentControls::controlForTest(const QString& key) const
{
    for (const Row& row : rows_) {
        if (row.key == key) {
            return row.control;
        }
    }
    if (key == QLatin1String("curve")) {
        return curve_;
    }
    if (key == QLatin1String("group")) {
        return groups_;
    }
    return nullptr;
}

} // namespace pictura

#include "moc_adjustment_controls.cpp"
