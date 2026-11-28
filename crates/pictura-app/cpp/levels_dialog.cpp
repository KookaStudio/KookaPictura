#include "levels_dialog.h"
#include <QtCore/QCoreApplication>
#include <QtCore/QtMath>

#include "adjustment_presets.h"
#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include <QtGui/QContextMenuEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QActionGroup>
#include <QtWidgets/QApplication>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

constexpr int kStripWidth = 256 + 2 * LevelsStrip::kInset;
constexpr int kThumbHeight = 10;

const QString kCustom = QStringLiteral("Custom");

const QStringList kChannelPrefixes = {QString(), QStringLiteral("red."),
                                      QStringLiteral("green."), QStringLiteral("blue.")};

// A level field over the engine layout's range for `name`; the input pair's
// limits are then coupled to each other at run time (`loadChannel`).
QSpinBox* levelSpin(QWidget* parent, const QString& name, const AdjustmentDialog::ParamSpec& range)
{
    auto* spin = new QSpinBox(parent);
    spin->setObjectName(name);
    spin->setRange(qRound(range.min), qRound(range.max));
    spin->setFixedWidth(AdjustmentDialog::kFieldWidth);
    spin->setKeyboardTracking(false);
    return spin;
}

} // namespace

LevelsStrip::LevelsStrip(bool ramp, int height, QWidget* parent)
    : QWidget(parent)
    , ramp_(ramp)
{
    setMinimumWidth(80);
    setFixedHeight(height);
}

void LevelsStrip::setBins(const std::array<int, 256>& bins)
{
    bins_ = bins;
    update();
}

void LevelsStrip::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const QRect face(kInset, 0, std::max(1, width() - 2 * kInset), height());
    if (ramp_) {
        QLinearGradient ramp(face.topLeft(), face.topRight());
        ramp.setColorAt(0.0, Qt::black);
        ramp.setColorAt(1.0, Qt::white);
        p.fillRect(face, ramp);
    } else {
        p.fillRect(face, Qt::white);
        const int peak = std::max(1, *std::max_element(bins_.begin(), bins_.end()));
        p.setPen(Qt::black);
        for (int i = 0; i < 256; ++i) {
            const int bar = int(double(bins_[i]) / peak * (face.height() - 1));
            if (bar > 0) {
                const int x = face.left() + i * (face.width() - 1) / 255;
                p.drawLine(x, face.bottom(), x, face.bottom() - bar + 1);
            }
        }
    }
    p.setPen(QColor(150, 150, 150));
    p.drawRect(face.adjusted(0, 0, -1, -1));
}

TriangleSlider::TriangleSlider(int count, QWidget* parent)
    : QWidget(parent)
    , thumbs_(count)
{
    setMinimumWidth(80);
    setFixedHeight(kThumbHeight + 2);
}

void TriangleSlider::setRange(int index, int min, int max)
{
    Thumb& thumb = thumbs_[index];
    thumb.min = min;
    thumb.max = std::max(min, max);
    thumb.value = std::clamp(thumb.value, thumb.min, thumb.max);
    update();
}

void TriangleSlider::setValue(int index, int value)
{
    Thumb& thumb = thumbs_[index];
    thumb.value = std::clamp(value, thumb.min, thumb.max);
    update();
}

void TriangleSlider::setColor(int index, const QColor& color)
{
    thumbs_[index].color = color;
    update();
}

int TriangleSlider::value(int index) const { return thumbs_[index].value; }

void TriangleSlider::setTrackInsets(int left, int right)
{
    leftInset_ = left;
    rightInset_ = right;
    update();
}

void TriangleSlider::setInvertedAppearance(bool inverted)
{
    inverted_ = inverted;
    update();
}

int TriangleSlider::xForValue(int value) const
{
    const int span = std::max(1, width() - leftInset_ - rightInset_);
    return leftInset_ + (inverted_ ? 255 - value : value) * span / 255;
}

int TriangleSlider::valueForX(int index, int x) const
{
    const int span = std::max(1, width() - leftInset_ - rightInset_);
    const int value = (x - leftInset_) * 255 / span;
    return std::clamp(inverted_ ? 255 - value : value, thumbs_[index].min, thumbs_[index].max);
}

void TriangleSlider::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    for (int i = 0; i < thumbs_.size(); ++i) {
        const auto& thumb = thumbs_[i];
        const qreal x = xForValue(thumb.value) + 0.5;
        QPainterPath path;
        path.moveTo(x, 1);
        path.lineTo(x - 5, kThumbHeight + 1);
        path.lineTo(x + 5, kThumbHeight + 1);
        path.closeSubpath();
        p.setBrush(thumb.color);
        const bool focused = hasFocus() && i == keyboardThumb_;
        p.setPen(QPen(focused ? palette().highlight().color() : QColor(90, 90, 90),
                      focused ? 2 : 1));
        p.drawPath(path);
    }
}

void TriangleSlider::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        event->ignore();
        return;
    }
    endDrag();
    int best = 12;
    // Ties go to the last thumb, so white can leave a black parked under it.
    for (int i = 0; i < thumbs_.size(); ++i) {
        const int distance = std::abs(int(event->position().x()) - xForValue(thumbs_[i].value));
        if (distance <= best) {
            best = distance;
            dragging_ = i;
        }
    }
    if (dragging_ >= 0) {
        keyboardThumb_ = dragging_;
        emit dragStarted();
        update();
    }
    if (draggingInputEndpoint()) {
        qApp->installEventFilter(this);
    }
    mouseMoveEvent(event);
}

void TriangleSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ < 0) {
        return;
    }
    const int value = valueForX(dragging_, int(event->position().x()));
    if (value != thumbs_[dragging_].value) {
        thumbs_[dragging_].value = value;
        update();
        emit valueChanged(dragging_, value);
    }
    updateClipping(event->modifiers().testFlag(Qt::AltModifier));
}

void TriangleSlider::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        endDrag();
    }
}

void TriangleSlider::keyPressEvent(QKeyEvent* event)
{
    const int key = event->key();
    if (key == Qt::Key_Up || key == Qt::Key_Down) {
        const int delta = key == Qt::Key_Up ? -1 : 1;
        keyboardThumb_ = std::clamp(keyboardThumb_ + delta, 0, int(thumbs_.size()) - 1);
        update();
        return;
    }
    if (key == Qt::Key_Left || key == Qt::Key_Right) {
        const int index = keyboardThumb_;
        const int delta = ((key == Qt::Key_Right) != inverted_) ? 1 : -1;
        const auto& thumb = thumbs_[index];
        const int value = std::clamp(thumb.value + delta, thumb.min, thumb.max);
        if (value != thumb.value) {
            emit dragStarted();
            setValue(index, value);
            emit valueChanged(index, value);
            emit dragEnded();
        }
        return;
    }
    QWidget::keyPressEvent(event);
}

bool TriangleSlider::event(QEvent* event)
{
    if (event->type() == QEvent::UngrabMouse || event->type() == QEvent::Hide) {
        endDrag();
    }
    return QWidget::event(event);
}

bool TriangleSlider::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() == QEvent::WindowDeactivate && watched == window()) {
        endDrag();
    } else if (event->type() == QEvent::KeyPress || event->type() == QEvent::KeyRelease) {
        auto* key = static_cast<QKeyEvent*>(event);
        if (key->key() == Qt::Key_Alt && !key->isAutoRepeat()) {
            updateClipping(event->type() == QEvent::KeyPress);
        }
    }
    return QWidget::eventFilter(watched, event);
}

bool TriangleSlider::draggingInputEndpoint() const
{
    return thumbs_.size() == 3 && (dragging_ == 0 || dragging_ == 2);
}

void TriangleSlider::updateClipping(bool alt)
{
    if (draggingInputEndpoint() && (alt || clipping_)) {
        clipping_ = alt;
        emit clippingThumbChanged(alt ? dragging_ : -1);
    }
}

void TriangleSlider::endDrag()
{
    if (draggingInputEndpoint()) {
        qApp->removeEventFilter(this);
        updateClipping(false);
    }
    const bool wasDragging = dragging_ >= 0;
    dragging_ = -1;
    if (wasDragging) {
        emit dragEnded();
    }
}

LevelsDialog::LevelsDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Levels"), parent)
{
    if (view) {
        const QString mode = image_mode(*view);
        grayscale_ = mode == QLatin1String("grayscale");
        canvasControlsAvailable_ = (grayscale_ || mode == QLatin1String("rgb"))
            && mask_edit_target_get(*view).isEmpty();
        connect(view, &PictureView::pyramid_settled, this, &LevelsDialog::renderClipping);
    }

    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);
    auto* left = new QVBoxLayout;
    left->setSpacing(4);

    auto* presetRow = new QHBoxLayout;
    presetRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Preset:"), this));
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("levelsPreset"));
    preset_->addItem(QStringLiteral("Default"));
    preset_->insertSeparator(preset_->count());
    for (const BuiltinPreset& preset : builtinPresets(QStringLiteral("levels"))) {
        preset_->addItem(preset.name);
    }
    const QStringList saved = savedPresetNames(QStringLiteral("levels"));
    if (!saved.isEmpty()) {
        preset_->insertSeparator(preset_->count());
        preset_->addItems(saved);
    }
    preset_->insertSeparator(preset_->count());
    preset_->addItem(kCustom);
    presetRow->addWidget(preset_, 1);
    left->addLayout(presetRow);

    auto* channelRow = new QHBoxLayout;
    channelRow->addSpacing(14);
    channelRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Channel:"), this));
    channel_ = new QComboBox(this);
    channel_->setObjectName(QStringLiteral("levelsChannel"));
    channel_->addItems({QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                        QStringLiteral("Blue")});
    channelRow->addWidget(channel_, 1);
    left->addLayout(channelRow);
    left->addSpacing(6);

    left->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Input Levels:"), this));
    histogram_ = new LevelsStrip(false, 120, this);
    histogram_->setFixedWidth(kStripWidth);
    histogram_->setObjectName(QStringLiteral("levelsHistogram"));
    left->addWidget(histogram_);
    input_ = new TriangleSlider(3, this);
    input_->setFixedWidth(kStripWidth);
    input_->setObjectName(QStringLiteral("levelsInputSlider"));
    input_->setColor(0, Qt::black);
    input_->setColor(1, QColor(128, 128, 128));
    input_->setColor(2, Qt::white);
    left->addWidget(input_);
    auto* inputRow = new QHBoxLayout;
    const auto levelField = [this](const QString& name) { return levelSpin(this, name, spec(name)); };
    inBlack_ = levelField(QStringLiteral("inputBlack"));
    gamma_ = new QDoubleSpinBox(this);
    gamma_->setObjectName(QStringLiteral("gamma"));
    const ParamSpec gamma = spec(QStringLiteral("gamma"));
    gamma_->setRange(gamma.min, gamma.max);
    gamma_->setDecimals(gamma.decimals);
    gamma_->setSingleStep(0.01);
    gamma_->setFixedWidth(kFieldWidth);
    gamma_->setKeyboardTracking(false);
    inWhite_ = levelField(QStringLiteral("inputWhite"));
    inputRow->addWidget(inBlack_);
    inputRow->addStretch(1);
    inputRow->addWidget(gamma_);
    inputRow->addStretch(1);
    inputRow->addWidget(inWhite_);
    left->addLayout(inputRow);
    left->addSpacing(10);

    left->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Output Levels:"), this));
    auto* outputRamp = new LevelsStrip(true, 16, this);
    outputRamp->setFixedWidth(kStripWidth);
    left->addWidget(outputRamp);
    output_ = new TriangleSlider(2, this);
    output_->setFixedWidth(kStripWidth);
    output_->setObjectName(QStringLiteral("levelsOutputSlider"));
    output_->setColor(0, Qt::black);
    output_->setColor(1, Qt::white);
    left->addWidget(output_);
    auto* outputRow = new QHBoxLayout;
    outBlack_ = levelField(QStringLiteral("outputBlack"));
    outWhite_ = levelField(QStringLiteral("outputWhite"));
    outputRow->addWidget(outBlack_);
    outputRow->addStretch(1);
    outputRow->addWidget(outWhite_);
    left->addLayout(outputRow);
    left->addStretch(1);
    // The fields line up under the strips, so the column is the strip's width.
    auto* leftBox = new QWidget(this);
    leftBox->setLayout(left);
    left->setContentsMargins(0, 0, 0, 0);
    leftBox->setFixedWidth(kStripWidth + 12);
    outer->addWidget(leftBox);

    QVBoxLayout* buttons =
        buttonColumn({QStringLiteral("Auto"), QStringLiteral("Options…")});
    button(QStringLiteral("Options…"))->setEnabled(false);
    buttons->addSpacing(10);
    buttons->addLayout(makePointControls());
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    connect(inBlack_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("inputBlack"), v); });
    connect(inWhite_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("inputWhite"), v); });
    connect(gamma_, &QDoubleSpinBox::valueChanged, this,
            [this](double v) { edit(QStringLiteral("gamma"), v); });
    connect(outBlack_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("outputBlack"), v); });
    connect(outWhite_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("outputWhite"), v); });
    connect(input_, &TriangleSlider::valueChanged, this, [this](int index, int value) {
        if (index == 0) {
            inBlack_->setValue(value);
        } else if (index == 2) {
            inWhite_->setValue(value);
        } else {
            // The gray thumb sits where the curve crosses half output:
            // norm^(1/gamma) = 0.5, so gamma = log(norm) / log(0.5).
            const int lo = inBlack_->value();
            const int hi = inWhite_->value();
            const double norm = std::clamp(double(value - lo) / std::max(1, hi - lo), 0.001, 0.999);
            gamma_->setValue(std::log(norm) / std::log(0.5));
        }
    });
    connect(input_, &TriangleSlider::clippingThumbChanged,
            this, &LevelsDialog::setClippingThumb);
    connect(output_, &TriangleSlider::valueChanged, this, [this](int index, int value) {
        (index == 0 ? outBlack_ : outWhite_)->setValue(value);
    });
    connect(channel_, &QComboBox::currentIndexChanged, this, &LevelsDialog::loadChannel);
    connect(preset_, &QComboBox::activated, this, &LevelsDialog::applyPreset);

    loadChannel();
}

LevelsDialog::~LevelsDialog()
{
    disarmPoint();
    clearClipping();
}

void LevelsDialog::done(int result)
{
    disarmPoint();
    clearClipping();
    QDialog::done(result);
}

QHBoxLayout* LevelsDialog::makePointControls()
{
    const std::array<QString, 3> labels{
        tr("Set Black Point"), tr("Set Gray Point"), tr("Set White Point")};
    const std::array<QString, 3> captions{tr("Black"), tr("Gray"), tr("White")};
    const std::array<QString, 3> names{QStringLiteral("levelsBlackPoint"),
                                      QStringLiteral("levelsGrayPoint"),
                                      QStringLiteral("levelsWhitePoint")};
    auto* row = new QHBoxLayout;
    auto* group = new QButtonGroup(this);
    for (int i = BlackPoint; i <= WhitePoint; ++i) {
        const auto kind = static_cast<PointKind>(i);
        auto* point = new QToolButton(this);
        point->setObjectName(names[i]);
        point->setToolTip(labels[i]);
        point->setAccessibleName(labels[i]);
        point->setIcon(icon(QStringLiteral("tool.eyedropper")));
        point->setText(captions[i]);
        point->setToolButtonStyle(Qt::ToolButtonTextUnderIcon);
        point->setCheckable(true);
        point->setEnabled(canvasControlsAvailable_ && !(grayscale_ && kind == GrayPoint));
        point->installEventFilter(this);
        group->addButton(point);
        row->addWidget(point);
        points_[i] = point;

        connect(point, &QToolButton::clicked, this, [this, kind] { armPoint(kind); });
    }
    return row;
}

bool LevelsDialog::eventFilter(QObject* watched, QEvent* event)
{
    for (int i = BlackPoint; i <= WhitePoint; ++i) {
        if (watched != points_[i] || !points_[i]->isEnabled()) {
            continue;
        }
        const auto kind = static_cast<PointKind>(i);
        if (event->type() == QEvent::ContextMenu) {
            showSampleMenu(kind, static_cast<QContextMenuEvent*>(event)->globalPos());
            return true;
        }
        if (event->type() == QEvent::MouseButtonDblClick
            && static_cast<QMouseEvent*>(event)->button() == Qt::LeftButton) {
            chooseTarget(kind);
            return true;
        }
    }
    return AdjustmentDialog::eventFilter(watched, event);
}

void LevelsDialog::armPoint(PointKind kind)
{
    if (tools()) {
        pointArmed_ = true;
        tools()->setCanvasSampler(
            [this, kind](const QPointF& pos, Qt::KeyboardModifiers) { pickPoint(kind, pos); }, false);
    }
}

void LevelsDialog::disarmPoint()
{
    if (pointArmed_ && tools()) {
        tools()->setCanvasSampler({});
    }
    pointArmed_ = false;
}

void LevelsDialog::chooseTarget(PointKind kind)
{
    QInputDialog dialog(this);
    dialog.setObjectName(QStringLiteral("levelsPointTarget"));
    dialog.setWindowTitle(points_[kind]->accessibleName());
    dialog.setLabelText(tr("Neutral target (R = G = B)"));
    dialog.setInputMode(QInputDialog::IntInput);
    dialog.setIntRange(kind == GrayPoint ? 1 : 0, kind == GrayPoint ? 254 : 255);
    dialog.setIntValue(targets_[kind]);
    if (dialog.exec() == QDialog::Accepted) {
        targets_[kind] = dialog.intValue();
    }
}

void LevelsDialog::showSampleMenu(PointKind kind, const QPoint& globalPos)
{
    auto* point = points_[kind];
    auto* menu = point->findChild<QMenu*>(QStringLiteral("levelsSampleSize"));
    if (!menu) {
        menu = new QMenu(point);
        menu->setObjectName(QStringLiteral("levelsSampleSize"));
        auto* group = new QActionGroup(menu);
        for (int size : {1, 3, 5, 11, 31, 51, 101}) {
            auto* action = menu->addAction(size == 1 ? tr("Point Sample")
                                                     : tr("%1 × %1 Average").arg(size));
            action->setCheckable(true);
            action->setData(size);
            group->addAction(action);
            connect(action, &QAction::triggered, this, [this, size] { sampleSize_ = size; });
        }
        menu->addSeparator();
        connect(menu->addAction(tr("Set Target…")), &QAction::triggered,
                this, [this, kind] { chooseTarget(kind); });
    }
    for (auto* action : menu->actions()) {
        if (action->data().isValid()) {
            action->setChecked(action->data().toInt() == sampleSize_);
        }
    }
    menu->popup(globalPos);
}

void LevelsDialog::pickPoint(PointKind kind, const QPointF& pos)
{
    if (!pictureView()) {
        return;
    }
    setPreviewSuspended(true);
    const QRgb sample = image_adjust_sample(*pictureView(), qFloor(pos.x()), qFloor(pos.y()),
                                            sampleSize_);
    setPreviewSuspended(false);
    if (qAlpha(sample) == 0) {
        return;
    }

    const std::array<int, 4> values{qGray(sample), qRed(sample), qGreen(sample), qBlue(sample)};
    QList<QPair<QString, double>> edits;
    const int first = grayscale_ ? 0 : 1;
    const int end = grayscale_ ? 1 : 4;
    for (int channel = first; channel < end; ++channel) {
        const QString& key = kChannelPrefixes.at(channel);
        const int value = values[channel];
        switch (kind) {
        case BlackPoint:
            if (value > 253) {
                return;
            }
            edits.append({key + QStringLiteral("inputBlack"), value});
            edits.append({key + QStringLiteral("outputBlack"), targets_[kind]});
            break;
        case WhitePoint:
            if (value < 2) {
                return;
            }
            edits.append({key + QStringLiteral("inputWhite"), value});
            edits.append({key + QStringLiteral("outputWhite"), targets_[kind]});
            break;
        case GrayPoint:
            if (value == 0 || value == 255) {
                return;
            }
            const double gamma = std::log(value / 255.0) / std::log(targets_[kind] / 255.0);
            edits.append({key + QStringLiteral("gamma"), std::clamp(gamma, 0.1, 9.99)});
            break;
        }
    }
    const QByteArray identity = defaultPresetBlock(QStringLiteral("levels"));
    if (setParams(identity, edits)) {
        markCustom();
        loadChannel();
    }
}

void LevelsDialog::setClippingCanvas(ImageView* canvas)
{
    if (canvas_) {
        disconnect(canvas_, &ImageView::viewChanged, this, &LevelsDialog::renderClipping);
        setClippingThumb(-1);
    }
    canvas_ = canvas;
    if (canvas_) {
        connect(canvas_, &ImageView::viewChanged, this, &LevelsDialog::renderClipping);
    }
}

void LevelsDialog::setClippingThumb(int index)
{
    if (index < 0) {
        if (clippingThumb_ >= 0) {
            clearClipping();
            setPreviewSuspended(false);
        }
        return;
    }
    if (!canvas_ || !canvasControlsAvailable_) {
        return;
    }
    clippingThumb_ = index;
    setPreviewSuspended(true);
    renderClipping();
}

void LevelsDialog::clearClipping()
{
    if (clippingThumb_ >= 0 && canvas_) {
        canvas_->clearFilterPreview();
    }
    clippingThumb_ = -1;
    clippingSource_ = QImage();
    clippingSourceCrop_ = QRect();
    clippingSourceLevel_ = -1;
}

void LevelsDialog::renderClipping()
{
    if (clippingThumb_ < 0 || !canvas_ || !pictureView()) {
        return;
    }
    const QRect visible = canvas_->visibleDocumentRect().toAlignedRect();
    if (visible.isEmpty()) {
        canvas_->clearFilterPreview();
        return;
    }
    constexpr int maxSide = 1536;
    const int available = document_api::levelCount(*pictureView());
    const int span = std::max(visible.width(), visible.height());
    int level = 0;
    int step = 1;
    while (span / step + 2 > maxSide && level < 30) {
        ++level;
        step *= 2;
    }
    if (span / step + 2 > maxSide) {
        canvas_->clearFilterPreview();
        return;
    }

    // The same power-of-two grid must locate both the source crop and overlay.
    const QRect crop(QPoint(visible.left() / step, visible.top() / step),
                     QPoint(visible.right() / step, visible.bottom() / step));
    QImage source;
    if (level < available) {
        source = pictureView()->display_image(level, crop.x(), crop.y(), crop.width(), crop.height());
    } else {
        if (clippingSource_.isNull() || clippingSourceCrop_ != crop || clippingSourceLevel_ != level) {
            clippingSource_ = image_adjust_clipping_source(*pictureView(), level, crop.x(), crop.y(),
                                                           crop.width(), crop.height());
            clippingSourceCrop_ = crop;
            clippingSourceLevel_ = level;
        }
        source = clippingSource_;
    }
    QImage map = source.convertToFormat(QImage::Format_ARGB32);
    if (map.isNull()) {
        canvas_->clearFilterPreview();
        return;
    }
    const bool shadows = clippingThumb_ == 0;
    const int threshold = shadows ? inBlack_->value() : inWhite_->value();
    const int channel = grayscale_ ? 0 : channel_->currentIndex();
    const auto output = [shadows, threshold](int sample) {
        return (shadows ? sample > threshold : sample >= threshold) ? 255 : 0;
    };
    for (int row = 0; row < map.height(); ++row) {
        auto* pixels = reinterpret_cast<QRgb*>(map.scanLine(row));
        for (int col = 0; col < map.width(); ++col) {
            const QRgb pixel = pixels[col];
            std::array<int, 3> rgb{qRed(pixel), qGreen(pixel), qBlue(pixel)};
            if (channel > 0) {
                rgb.fill(rgb.at(channel - 1));
            }
            pixels[col] = qRgba(output(rgb[0]), output(rgb[1]), output(rgb[2]), qAlpha(pixel));
        }
    }
    canvas_->setFilterPreview(
        map, QRect(crop.x() * step, crop.y() * step, crop.width() * step, crop.height() * step),
        true);
}

QString LevelsDialog::prefix() const { return kChannelPrefixes.value(channel_->currentIndex()); }

int LevelsDialog::gammaPosition(double gamma) const
{
    const int lo = inBlack_->value();
    const int hi = inWhite_->value();
    return lo + qRound(std::pow(0.5, gamma) * (hi - lo));
}

void LevelsDialog::loadChannel()
{
    loading_ = true;
    const QString key = prefix();
    const int ib = qRound(param(key + QStringLiteral("inputBlack"), 0));
    const int iw = qRound(param(key + QStringLiteral("inputWhite"), 255));
    inBlack_->setMaximum(iw - 2);
    inWhite_->setMinimum(ib + 2);
    inBlack_->setValue(ib);
    inWhite_->setValue(iw);
    gamma_->setValue(param(key + QStringLiteral("gamma"), 1.0));
    outBlack_->setValue(qRound(param(key + QStringLiteral("outputBlack"), 0)));
    outWhite_->setValue(qRound(param(key + QStringLiteral("outputWhite"), 255)));

    input_->setRange(0, 0, iw - 2);
    input_->setRange(1, ib + 1, iw - 1);
    input_->setRange(2, ib + 2, 255);
    input_->setValue(0, ib);
    input_->setValue(2, iw);
    input_->setValue(1, gammaPosition(gamma_->value()));
    output_->setRange(0, 0, 255);
    output_->setRange(1, 0, 255);
    output_->setValue(0, outBlack_->value());
    output_->setValue(1, outWhite_->value());
    histogram_->setBins(histogram(channel_->currentIndex()));
    loading_ = false;
}

void LevelsDialog::edit(const QString& name, double value)
{
    if (loading_) {
        return;
    }
    setParam(prefix() + name, value);
    markCustom();
    // Reload even after a good edit: the black / white limits and the gray
    // thumb follow every input change.
    loadChannel();
}

void LevelsDialog::applyPreset(int index)
{
    const QString name = preset_->itemText(index);
    QByteArray next;
    if (name == QLatin1String("Default")) {
        next = defaultPresetBlock(QStringLiteral("levels"));
    } else {
        for (const BuiltinPreset& preset : builtinPresets(QStringLiteral("levels"))) {
            if (preset.name == name) {
                next = preset.build(block());
                break;
            }
        }
        if (next.isEmpty() && savedPresetNames(QStringLiteral("levels")).contains(name)) {
            next = savedPresetBlock(QStringLiteral("levels"), name);
        }
    }
    if (next.isEmpty() || !setBlock(next)) {
        return;
    }
    const QSignalBlocker block(preset_);
    channel_->setCurrentIndex(0);
    loadChannel();
}

void LevelsDialog::applyAuto()
{
    int lo = 0;
    int hi = 0;
    if (!autoEndpoints(channel_->currentIndex(), lo, hi)) {
        return;
    }
    const QString key = prefix();
    if (!setParams({{key + QStringLiteral("inputBlack"), 0.0},
                    {key + QStringLiteral("inputWhite"), double(hi)},
                    {key + QStringLiteral("inputBlack"), double(lo)}})) {
        return;
    }
    markCustom();
    loadChannel();
}

void LevelsDialog::markCustom()
{
    const QSignalBlocker block(preset_);
    preset_->setCurrentIndex(preset_->findText(kCustom));
}

QWidget* LevelsDialog::controlForTest(const QString& key) const
{
    for (QWidget* widget : {static_cast<QWidget*>(preset_), static_cast<QWidget*>(channel_),
                            static_cast<QWidget*>(input_), static_cast<QWidget*>(output_),
                            static_cast<QWidget*>(inBlack_), static_cast<QWidget*>(gamma_),
                            static_cast<QWidget*>(inWhite_), static_cast<QWidget*>(outBlack_),
                            static_cast<QWidget*>(outWhite_)}) {
        if (widget->objectName() == key) {
            return widget;
        }
    }
    return nullptr;
}

} // namespace pictura

#include "moc_levels_dialog.cpp"
