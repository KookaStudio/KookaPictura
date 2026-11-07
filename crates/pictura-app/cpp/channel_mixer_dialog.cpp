#include "channel_mixer_dialog.h"

#include "adjustment_presets.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

const QColor kDark(70, 70, 70);

} // namespace

ChannelMixerDialog::ChannelMixerDialog(PictureView* view, const QByteArray& block,
                                       const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Channel Mixer"), parent)
{
    const bool cmyk = this->cmyk();
    sources_ = cmyk ? QStringList{QStringLiteral("cyan"), QStringLiteral("magenta"),
                                  QStringLiteral("yellow"), QStringLiteral("black")}
                    : QStringList{QStringLiteral("red"), QStringLiteral("green"),
                                  QStringLiteral("blue")};
    const QStringList labels = cmyk ? QStringList{QStringLiteral("Cyan:"),
                                                  QStringLiteral("Magenta:"),
                                                  QStringLiteral("Yellow:"),
                                                  QStringLiteral("Black:")}
                                    : QStringList{QStringLiteral("Red:"),
                                                  QStringLiteral("Green:"),
                                                  QStringLiteral("Blue:")};
    const QColor colors[] = {QColor(0, 174, 239), QColor(236, 0, 140), QColor(255, 242, 0), kDark};

    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    left->setSpacing(8);
    addPresetRow(left);

    auto* outputRow = new QHBoxLayout;
    outputRow->addWidget(new QLabel(QStringLiteral("Output Channel:"), this));
    output_ = new QComboBox(this);
    output_->setObjectName(QStringLiteral("outputChannel"));
    output_->setMinimumWidth(110);
    outputRow->addWidget(output_);
    outputRow->addStretch(1);
    left->addLayout(outputRow);

    auto* sources = new QGroupBox(QStringLiteral("Source Channels"), this);
    auto* grid = new QGridLayout(sources);
    grid->setColumnStretch(1, 1);
    grid->setVerticalSpacing(4);
    for (int i = 0; i < sources_.size(); ++i) {
        addStackedRow(grid, i * 3, labels.at(i), sources_.at(i), 14, {kDark, colors[i]},
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
    grid->addLayout(totalRow, sources_.size() * 3, 0, 1, 3);
    left->addWidget(sources);

    if (!cmyk) {
        monochrome_ = new QCheckBox(QStringLiteral("Monochrome"), this);
        monochrome_->setObjectName(QStringLiteral("monochrome"));
        monochrome_->setChecked(param(QStringLiteral("monochrome")) != 0);
        left->addWidget(monochrome_);
        connect(monochrome_, &QCheckBox::toggled, this, &ChannelMixerDialog::toggleMonochrome);
    }

    auto* constant = new QGridLayout;
    constant->setColumnStretch(1, 1);
    addStackedRow(constant, 0, QStringLiteral("Constant:"), QStringLiteral("constant"), 0, {},
                  QStringLiteral(" %"));
    left->addLayout(constant);
    left->addStretch(1);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn();
    buttons->addSpacing(10);
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    for (const QString& key : sources_) {
        connect(findChild<QSpinBox*>(key), &QSpinBox::valueChanged, this,
                &ChannelMixerDialog::updateTotal);
    }
    connect(output_, &QComboBox::currentIndexChanged, this, &ChannelMixerDialog::loadOutput);
    syncOutputs();
    setMinimumWidth(cmyk ? 520 : 460);
    loadOutput();
}

QString ChannelMixerDialog::paramKey(const QString& key) const
{
    return QString::number(output_ ? std::max(0, output_->currentIndex()) : 0) + QLatin1Char('.')
           + key;
}

void ChannelMixerDialog::syncOutputs()
{
    const int current = output_->currentIndex();
    QSignalBlocker blocker(output_);
    output_->clear();
    for (const QString& row : page()) {
        if (row.startsWith(QLatin1String("groups\t"))) {
            output_->addItems(row.section(QLatin1Char('\t'), 1).split(QLatin1Char('|')));
        }
    }
    output_->setCurrentIndex(std::min(std::max(0, current), output_->count() - 1));
}

void ChannelMixerDialog::reloadFromBlock()
{
    syncOutputs();
    loadOutput();
}

void ChannelMixerDialog::loadOutput()
{
    // Each field writes back the value just read: a no-op edit.
    for (const QString& key : sources_ + QStringList{QStringLiteral("constant")}) {
        findChild<QSpinBox*>(key)->setValue(qRound(param(paramKey(key))));
    }
    updateTotal();
}

void ChannelMixerDialog::toggleMonochrome(bool on)
{
    if (!setParam(QStringLiteral("monochrome"), on ? 1.0 : 0.0)) {
        QSignalBlocker blocker(monochrome_);
        monochrome_->setChecked(!on);
        return;
    }
    syncOutputs();
    loadOutput();
}

void ChannelMixerDialog::updateTotal()
{
    int total = 0;
    for (const QString& key : sources_) {
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
