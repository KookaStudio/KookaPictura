#include "pict_options_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {
constexpr int kResolutions[] = {16, 32};
const char* const kResolutionNames[] = {"pictOptionsResolution16", "pictOptionsResolution32"};
const char* const kCompressionNames[] = {
    "pictOptionsCompressionNone",
    "pictOptionsCompressionLow",
    "pictOptionsCompressionMedium",
    "pictOptionsCompressionHigh",
    "pictOptionsCompressionMaximum",
};
} // namespace

PictOptionsDialog::PictOptionsDialog(const SaveFormatOptions& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("pictOptionsDialog"));
    setWindowTitle(tr("PICT Options"));
    auto* root = new QVBoxLayout(this);

    auto* resolutionBox = new QGroupBox(tr("Resolution"), this);
    auto* resolutionLayout = new QVBoxLayout(resolutionBox);
    auto* resolutionGroup = new QButtonGroup(resolutionBox);
    for (int i = 0; i < 2; ++i) {
        resolution_[i] = new QRadioButton(tr("%1 bits/pixel").arg(kResolutions[i]), resolutionBox);
        resolution_[i]->setObjectName(QString::fromLatin1(kResolutionNames[i]));
        resolutionGroup->addButton(resolution_[i], kResolutions[i]);
        if (kResolutions[i] == initial.pictResolution) {
            resolution_[i]->setChecked(true);
        }
        resolutionLayout->addWidget(resolution_[i]);
    }
    if (!resolutionGroup->checkedButton()) {
        resolution_[1]->setChecked(true);
    }
    root->addWidget(resolutionBox);

    auto* compressionBox = new QGroupBox(tr("Compression"), this);
    auto* compressionLayout = new QVBoxLayout(compressionBox);
    auto* compressionGroup = new QButtonGroup(compressionBox);
    const char* const labels[] = {
        QT_TR_NOOP("None"),
        QT_TR_NOOP("JPEG Low"),
        QT_TR_NOOP("JPEG Medium"),
        QT_TR_NOOP("JPEG High"),
        QT_TR_NOOP("JPEG Maximum"),
    };
    for (int i = 0; i < 5; ++i) {
        compression_[i] = new QRadioButton(tr(labels[i]), compressionBox);
        compression_[i]->setObjectName(QString::fromLatin1(kCompressionNames[i]));
        compressionGroup->addButton(compression_[i], i);
        if (i == initial.pictCompression) {
            compression_[i]->setChecked(true);
        }
        compressionLayout->addWidget(compression_[i]);
    }
    if (!compressionGroup->checkedButton()) {
        compression_[0]->setChecked(true);
    }
    root->addWidget(compressionBox);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &PictOptionsDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &PictOptionsDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(320);
}

int PictOptionsDialog::resolution() const
{
    for (int i = 0; i < 2; ++i) {
        if (resolution_[i]->isChecked()) {
            return kResolutions[i];
        }
    }
    return 32;
}

int PictOptionsDialog::compression() const
{
    for (int i = 0; i < 5; ++i) {
        if (compression_[i]->isChecked()) {
            return i;
        }
    }
    return 0;
}

} // namespace pictura

#include "moc_pict_options_dialog.cpp"
