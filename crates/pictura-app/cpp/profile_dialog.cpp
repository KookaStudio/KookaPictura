#include "profile_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kSrgb = 0;
constexpr int kAdobeRgb = 1;
constexpr int kProPhotoRgb = 2;

} // namespace

ProfileDialog::ProfileDialog(bool convert, QWidget* parent)
    : ProfileDialog(convert, QString(), parent)
{
}

ProfileDialog::ProfileDialog(bool convert, const QString& sourceSpace, QWidget* parent)
    : QDialog(parent)
    , convert_(convert)
{
    buildUi(convert, sourceSpace);
}

void ProfileDialog::buildUi(bool convert, const QString& sourceSpace)
{
    setObjectName(QStringLiteral("profileDialog"));
    setWindowTitle(convert ? tr("Convert to Profile") : tr("Assign Profile"));

    auto* outer = new QVBoxLayout(this);
    if (convert && !sourceSpace.isEmpty()) {
        source_ = new QLabel(tr("Source Space: %1").arg(sourceSpace), this);
        source_->setObjectName(QStringLiteral("profileDialogSource"));
        outer->addWidget(source_);
    }

    combo_ = new QComboBox(this);
    combo_->setObjectName(QStringLiteral("profileDialogCombo"));
    if (convert) {
        combo_->addItem(tr("sRGB IEC61966-2.1"), kSrgb);
        combo_->addItem(tr("Adobe RGB (1998)"), kAdobeRgb);
        combo_->addItem(tr("Pro Photo RGB"), kProPhotoRgb);
    } else {
        combo_->addItem(tr("Don't Color Manage"), kSrgb);
        combo_->addItem(tr("Adobe RGB (1998)"), kAdobeRgb);
        combo_->addItem(tr("Pro Photo RGB"), kProPhotoRgb);
    }
    outer->addWidget(combo_);

    if (convert) {
        options_ = new QGroupBox(tr("Conversion Options"), this);
        options_->setObjectName(QStringLiteral("profileDialogOptions"));
        auto* form = new QFormLayout(options_);
        form->addRow(tr("Engine:"), new QLabel(QStringLiteral("Little CMS 2"), options_));
        intent_ = new QComboBox(options_);
        intent_->setObjectName(QStringLiteral("profileDialogIntent"));
        intent_->addItem(tr("Perceptual"), 0);
        intent_->addItem(tr("Relative Colorimetric"), 1);
        intent_->addItem(tr("Saturation"), 2);
        intent_->addItem(tr("Absolute Colorimetric"), 3);
        intent_->setCurrentIndex(intent_->findData(1));
        form->addRow(tr("Intent:"), intent_);
        bpc_ = new QCheckBox(tr("Use Black Point Compensation"), options_);
        bpc_->setObjectName(QStringLiteral("profileDialogBpc"));
        bpc_->setChecked(true);
        form->addRow(QString(), bpc_);
        dither_ = new QCheckBox(tr("Use Dither"), options_);
        dither_->setObjectName(QStringLiteral("profileDialogDither"));
        dither_->setChecked(true);
        form->addRow(QString(), dither_);
        outer->addWidget(options_);

        flatten_ = new QCheckBox(tr("Flatten Image"), this);
        flatten_->setObjectName(QStringLiteral("profileDialogFlatten"));
        outer->addWidget(flatten_);
        preview_ = new QCheckBox(tr("Preview"), this);
        preview_->setObjectName(QStringLiteral("profileDialogPreview"));
        outer->addWidget(preview_);

        connect(combo_, &QComboBox::currentIndexChanged, this, [this]() {
            emit optionsChanged();
        });
        connect(intent_, &QComboBox::currentIndexChanged, this, [this]() {
            emit optionsChanged();
        });
        for (QCheckBox* box : {bpc_, dither_, flatten_, preview_}) {
            connect(box, &QCheckBox::toggled, this, [this]() { emit optionsChanged(); });
        }
    }

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &ProfileDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &ProfileDialog::reject);
    outer->addWidget(buttons);
    combo_->setFocus();
}

int ProfileDialog::profileIndex() const
{
    return combo_->currentData().toInt();
}

int ProfileDialog::renderIntent() const
{
    return intent_ ? intent_->currentData().toInt() : 1;
}

bool ProfileDialog::blackPointCompensation() const
{
    return bpc_ && bpc_->isChecked();
}

bool ProfileDialog::useDither() const
{
    return dither_ && dither_->isChecked();
}

bool ProfileDialog::flatten() const
{
    return flatten_ && flatten_->isChecked();
}

bool ProfileDialog::preview() const
{
    return preview_ && preview_->isChecked();
}

QStringList ProfileDialog::choicesForTest() const
{
    QStringList out;
    for (int i = 0; i < combo_->count(); ++i) {
        out.append(combo_->itemText(i));
    }
    return out;
}

QStringList ProfileDialog::intentChoicesForTest() const
{
    QStringList out;
    if (!intent_) {
        return out;
    }
    for (int i = 0; i < intent_->count(); ++i) {
        out.append(intent_->itemText(i));
    }
    return out;
}

void ProfileDialog::setChoiceForTest(int index)
{
    if (index >= 0 && index < combo_->count()) {
        combo_->setCurrentIndex(index);
    }
}

void ProfileDialog::setConversionOptions(int intent, bool bpc, bool dither)
{
    if (intent_) {
        const int index = intent_->findData(intent);
        if (index >= 0) {
            intent_->setCurrentIndex(index);
        }
    }
    if (bpc_) {
        bpc_->setChecked(bpc);
    }
    if (dither_) {
        dither_->setChecked(dither);
    }
}

void ProfileDialog::setPreviewForTest(bool on)
{
    if (preview_) {
        preview_->setChecked(on);
    }
}

} // namespace pictura

#include "moc_profile_dialog.cpp"
