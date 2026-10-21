#include "channel_mixer_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

const QStringList kSources = {QStringLiteral("red"), QStringLiteral("green"),
                              QStringLiteral("blue")};
const QString kConstant = QStringLiteral("constant");
const QColor kDark(70, 70, 70);

} // namespace

ChannelMixerDialog::ChannelMixerDialog(PictureView* view, const QByteArray& block,
                                       const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Channel Mixer"), parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    left->setSpacing(8);

    auto* outputRow = new QHBoxLayout;
    outputRow->addWidget(new QLabel(QStringLiteral("Output Channel:"), this));
    output_ = new QComboBox(this);
    output_->setObjectName(QStringLiteral("outputChannel"));
    for (const QString& row : page()) {
        if (row.startsWith(QLatin1String("groups\t"))) {
            output_->addItems(row.section(QLatin1Char('\t'), 1).split(QLatin1Char('|')));
        }
    }
    output_->setMinimumWidth(110);
    outputRow->addWidget(output_);
    outputRow->addStretch(1);
    left->addLayout(outputRow);

    auto* sources = new QGroupBox(QStringLiteral("Source Channels"), this);
    auto* grid = new QGridLayout(sources);
    grid->setColumnStretch(1, 1);
    grid->setVerticalSpacing(4);
    const QColor colors[] = {QColor(230, 40, 40), QColor(40, 190, 60), QColor(50, 80, 230)};
    for (int i = 0; i < 3; ++i) {
        const QString label = QStringList{QStringLiteral("Red:"), QStringLiteral("Green:"),
                                          QStringLiteral("Blue:")}[i];
        addStackedRow(grid, i * 3, label, kSources[i], -200, 200, 14, {kDark, colors[i]},
                      QStringLiteral(" %"));
        grid->setRowMinimumHeight(i * 3 + 2, 6);
    }
    auto* totalRow = new QHBoxLayout;
    totalRow->addStretch(1);
    totalRow->addWidget(new QLabel(QStringLiteral("Total:"), sources));
    totalRow->addSpacing(8);
    total_ = new QLabel(sources);
    total_->setObjectName(QStringLiteral("channelMixerTotal"));
    total_->setFixedWidth(kFieldWidth);
    totalRow->addWidget(total_);
    grid->addLayout(totalRow, 9, 0, 1, 3);
    left->addWidget(sources);

    auto* constant = new QGridLayout;
    constant->setColumnStretch(1, 1);
    addStackedRow(constant, 0, QStringLiteral("Constant:"), kConstant, -200, 200, 0, {},
                  QStringLiteral(" %"));
    left->addLayout(constant);
    left->addStretch(1);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn();
    buttons->addSpacing(10);
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    for (const QString& key : kSources) {
        connect(findChild<QSpinBox*>(key), &QSpinBox::valueChanged, this,
                &ChannelMixerDialog::updateTotal);
    }
    connect(output_, &QComboBox::currentIndexChanged, this, &ChannelMixerDialog::loadOutput);
    setMinimumWidth(460);
    updateTotal();
}

QString ChannelMixerDialog::paramKey(const QString& key) const
{
    return QString::number(output_ ? std::max(0, output_->currentIndex()) : 0) + QLatin1Char('.')
           + key;
}

void ChannelMixerDialog::loadOutput()
{
    // Each field writes back the value just read: a no-op edit.
    for (const QString& key : kSources + QStringList{kConstant}) {
        findChild<QSpinBox*>(key)->setValue(qRound(param(paramKey(key))));
    }
    updateTotal();
}

void ChannelMixerDialog::updateTotal()
{
    int total = 0;
    for (const QString& key : kSources) {
        total += findChild<QSpinBox*>(key)->value();
    }
    // CS6 flags a total over 100% (the output can clip) with a warning sign.
    total_->setText(QStringLiteral("%1%2 %")
                        .arg(total > 0 ? QStringLiteral("+") : QString())
                        .arg(total)
                    + (total > 100 ? QStringLiteral(" ⚠") : QString()));
}

QWidget* ChannelMixerDialog::controlForTest(const QString& key) const
{
    return findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_channel_mixer_dialog.cpp"
