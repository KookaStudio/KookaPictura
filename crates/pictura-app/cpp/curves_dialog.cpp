#include "curves_dialog.h"

#include "panels/curve_widget.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QSpinBox* pointSpin(QWidget* parent, const QString& name)
{
    auto* spin = new QSpinBox(parent);
    spin->setObjectName(name);
    spin->setRange(0, 255);
    spin->setFixedWidth(AdjustmentDialog::kFieldWidth);
    spin->setKeyboardTracking(false);
    return spin;
}

} // namespace

CurvesDialog::CurvesDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Curves"), parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(14, 12, 14, 12);
    layout->setSpacing(8);

    auto* channelRow = new QHBoxLayout;
    channelRow->addWidget(new QLabel(QStringLiteral("Channel:"), this));
    channel_ = new QComboBox(this);
    channel_->setObjectName(QStringLiteral("curvesChannel"));
    channel_->addItems({QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                        QStringLiteral("Blue")});
    channel_->setMinimumWidth(110);
    channelRow->addWidget(channel_);
    channelRow->addStretch(1);
    layout->addLayout(channelRow);

    curve_ = new CurveWidget(this);
    curve_->setObjectName(QStringLiteral("curve"));
    curve_->setShowRamps(true);
    curve_->setFixedSize(curve_->sizeHint() + QSize(32, 32));
    layout->addWidget(curve_, 0, Qt::AlignLeft);

    auto* pointRow = new QHBoxLayout;
    input_ = pointSpin(this, QStringLiteral("curvesInput"));
    output_ = pointSpin(this, QStringLiteral("curvesOutput"));
    pointRow->addWidget(new QLabel(QStringLiteral("Input:"), this));
    pointRow->addWidget(input_);
    pointRow->addSpacing(12);
    pointRow->addWidget(new QLabel(QStringLiteral("Output:"), this));
    pointRow->addWidget(output_);
    pointRow->addStretch(1);
    layout->addLayout(pointRow);

    auto* hint = new QLabel(
        QStringLiteral("Click to add a point · drag off or Ctrl-click or Delete to remove"), this);
    hint->setEnabled(false);
    layout->addWidget(hint);
    layout->addWidget(previewCheck());
    layout->addSpacing(4);

    layout->addLayout(bottomButtons());

    connect(channel_, &QComboBox::currentIndexChanged, this, &CurvesDialog::loadChannel);
    connect(curve_, &CurveWidget::curveChanged, this, [this]() {
        if (!setCurve(channel_->currentIndex(), curve_->pointsText())) {
            loadChannel();
        }
        showSelected();
    });
    connect(curve_, &CurveWidget::selectionChanged, this, &CurvesDialog::showSelected);
    const auto moved = [this]() { curve_->moveSelected(input_->value(), output_->value()); };
    connect(input_, &QSpinBox::valueChanged, this, moved);
    connect(output_, &QSpinBox::valueChanged, this, moved);

    loadChannel();
}

void CurvesDialog::loadChannel()
{
    curve_->setPointsText(curve(channel_->currentIndex()));
    curve_->setBins(histogram(channel_->currentIndex()));
    showSelected();
}

void CurvesDialog::showSelected()
{
    const int index = curve_->selected();
    const bool any = index >= 0 && index < curve_->points().size();
    const QPointF point = any ? curve_->points().at(index) : QPointF();
    for (QSpinBox* spin : {input_, output_}) {
        const QSignalBlocker block(spin);
        spin->setEnabled(any);
        spin->setValue(qRound((spin == input_ ? point.x() : point.y()) * 255.0));
    }
}

QWidget* CurvesDialog::controlForTest(const QString& key) const
{
    for (QWidget* widget : {static_cast<QWidget*>(channel_), static_cast<QWidget*>(curve_),
                            static_cast<QWidget*>(input_), static_cast<QWidget*>(output_)}) {
        if (widget->objectName() == key) {
            return widget;
        }
    }
    return nullptr;
}

} // namespace pictura

#include "moc_curves_dialog.cpp"
