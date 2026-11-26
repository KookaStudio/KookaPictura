#include "indexed_color_dialog.h"

#include <QtCore/QRegularExpression>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

IndexedColorOptions legacyIndexedOptions(bool exactAvailable)
{
    IndexedColorOptions options;
    options.palette = exactAvailable ? 0 : 2;
    return options;
}

} // namespace

IndexedColorDialog::IndexedColorDialog(bool exactAvailable, QWidget* parent)
    : IndexedColorDialog(exactAvailable, legacyIndexedOptions(exactAvailable), parent)
{
}

IndexedColorDialog::IndexedColorDialog(bool exactAvailable, bool documentOptions, QWidget* parent)
    : IndexedColorDialog(exactAvailable, legacyIndexedOptions(exactAvailable), parent,
                         documentOptions)
{
}

IndexedColorDialog::IndexedColorDialog(bool exactAvailable, const IndexedColorOptions& initial,
                                       QWidget* parent, bool documentOptions)
    : QDialog(parent)
{
    setWindowTitle(tr("Indexed Color"));
    auto* root = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    auto* paletteBox = new QGroupBox(tr("Palette"), this);
    auto* paletteForm = new QFormLayout(paletteBox);
    palette_ = new QComboBox(paletteBox);
    palette_->setObjectName(QStringLiteral("indexedPalette"));
    if (exactAvailable) {
        palette_->addItem(tr("Exact"), 0);
    }
    palette_->addItem(tr("Web"), 1);
    palette_->addItem(tr("Local (Perceptual)"), 2);
    palette_->addItem(tr("Local (Selective)"), 3);
    palette_->addItem(tr("Local (Adaptive)"), 4);
    const int wanted = palette_->findData(initial.palette);
    palette_->setCurrentIndex(wanted >= 0 ? wanted : palette_->findData(2));
    paletteForm->addRow(tr("Palette:"), palette_);
    colors_ = new QSpinBox(paletteBox);
    colors_->setObjectName(QStringLiteral("indexedColors"));
    colors_->setRange(2, 256);
    colors_->setValue(std::clamp(initial.colors, 2, 256));
    paletteForm->addRow(tr("Colors:"), colors_);
    if (documentOptions) {
        auto* forceRow = new QHBoxLayout;
        forced_ = new QComboBox(paletteBox);
        forced_->setObjectName(QStringLiteral("indexedForced"));
        forced_->addItems({tr("None"), tr("Black And White"), tr("Primaries"), tr("Web"),
                           tr("Custom")});
        forceRow->addWidget(forced_);
        auto* edit = new QPushButton(tr("Edit…"), paletteBox);
        edit->setEnabled(false);
        forceRow->addWidget(edit);
        paletteForm->addRow(tr("Forced:"), forceRow);
        connect(forced_, &QComboBox::currentIndexChanged, this, [this, edit](int index) {
            edit->setEnabled(index == 4);
            emit optionsChanged();
        });
        connect(edit, &QPushButton::clicked, this, &IndexedColorDialog::editForcedColors);

        transparency_ = new QCheckBox(tr("Transparency"), paletteBox);
        transparency_->setObjectName(QStringLiteral("indexedTransparency"));
        transparency_->setChecked(true);
        paletteForm->addRow(transparency_);
        connect(transparency_, &QCheckBox::toggled, this, &IndexedColorDialog::optionsChanged);
    }
    left->addWidget(paletteBox);

    auto* optionsBox = new QGroupBox(tr("Options"), this);
    auto* optionsForm = new QFormLayout(optionsBox);
    dither_ = new QComboBox(optionsBox);
    dither_->setObjectName(QStringLiteral("indexedDither"));
    dither_->addItem(tr("None"), 0);
    dither_->addItem(tr("Diffusion"), 1);
    dither_->addItem(tr("Pattern"), 2);
    dither_->addItem(tr("Noise"), 3);
    const int wantedDither = dither_->findData(initial.dither);
    dither_->setCurrentIndex(wantedDither >= 0 ? wantedDither : 1);
    optionsForm->addRow(tr("Dither:"), dither_);
    amount_ = new QSpinBox(optionsBox);
    amount_->setObjectName(QStringLiteral("indexedAmount"));
    amount_->setRange(0, 100);
    amount_->setValue(std::clamp(initial.amount, 0, 100));
    amount_->setSuffix(QStringLiteral(" %"));
    optionsForm->addRow(tr("Amount:"), amount_);
    left->addWidget(optionsBox);
    left->addStretch();
    root->addLayout(left);

    auto* right = new QVBoxLayout;
    auto* buttons =
        new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, Qt::Vertical, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    right->addWidget(buttons);
    preview_ = new QCheckBox(tr("Preview"), this);
    preview_->setObjectName(QStringLiteral("indexedPreview"));
    preview_->setChecked(initial.preview);
    right->addWidget(preview_);
    right->addStretch();
    root->addLayout(right);

    const auto changed = [this]() {
        syncEnabled();
        emit optionsChanged();
    };
    connect(palette_, &QComboBox::currentIndexChanged, this, changed);
    connect(colors_, &QSpinBox::valueChanged, this, changed);
    connect(dither_, &QComboBox::currentIndexChanged, this, changed);
    connect(amount_, &QSpinBox::valueChanged, this, changed);
    connect(preview_, &QCheckBox::toggled, this, changed);
    syncEnabled();
}

// Exact and Web fix the table (all colors / the 216 web colors); Exact also has
// nothing to dither, and None has no amount.
void IndexedColorDialog::syncEnabled()
{
    const int p = palette();
    colors_->setEnabled(p >= 2);
    dither_->setEnabled(p != 0);
    amount_->setEnabled(p != 0 && dither() != 0);
}

void IndexedColorDialog::setExactAvailable(bool available)
{
    const int index = palette_->findData(0);
    if (available == (index >= 0)) {
        return;
    }
    const QSignalBlocker blocker(palette_);
    const int selected = palette();
    if (available) {
        palette_->insertItem(0, tr("Exact"), 0);
        palette_->setCurrentIndex(palette_->findData(selected));
    } else {
        palette_->removeItem(index);
        palette_->setCurrentIndex(palette_->findData(selected == 0 ? 4 : selected));
    }
    syncEnabled();
}

bool IndexedColorDialog::transparency() const
{
    return transparency_ && transparency_->isChecked();
}

QByteArray IndexedColorDialog::forcedColors() const
{
    switch (forced_ ? forced_->currentIndex() : 0) {
    case 1:
        return QByteArray::fromHex("000000ffffff");
    case 2:
        return QByteArray::fromHex("000000" "ffffff" "ff0000" "00ff00"
                                   "0000ff" "00ffff" "ff00ff" "ffff00");
    case 3: {
        QByteArray colors;
        colors.reserve(216 * 3);
        for (int red = 0; red <= 255; red += 51) {
            for (int green = 0; green <= 255; green += 51) {
                for (int blue = 0; blue <= 255; blue += 51) {
                    colors.append(static_cast<char>(red));
                    colors.append(static_cast<char>(green));
                    colors.append(static_cast<char>(blue));
                }
            }
        }
        return colors;
    }
    case 4:
        return customForced_;
    default:
        return {};
    }
}

void IndexedColorDialog::editForcedColors()
{
    QStringList previous;
    for (qsizetype i = 0; i < customForced_.size(); i += 3) {
        previous.append(QString::fromLatin1(customForced_.mid(i, 3).toHex()));
    }
    bool accepted = false;
    const QString input = QInputDialog::getMultiLineText(
        this, tr("Forced Colors"), tr("Enter one RGB hex color per line."),
        previous.join(QLatin1Char('\n')), &accepted);
    if (!accepted) {
        return;
    }
    const QRegularExpression whitespace(QStringLiteral("\\s+"));
    const QRegularExpression hexColor(QStringLiteral("^#?[0-9a-fA-F]{6}$"));
    const auto entries = input.split(whitespace, Qt::SkipEmptyParts);
    QByteArray colors;
    for (const auto& entry : entries) {
        if (!hexColor.match(entry).hasMatch()) {
            QMessageBox::warning(this, tr("Forced Colors"),
                                 tr("Use six hexadecimal digits for each color."));
            return;
        }
        if (entry.startsWith(QLatin1Char('#'))) {
            colors += QByteArray::fromHex(entry.mid(1).toLatin1());
        } else {
            colors += QByteArray::fromHex(entry.toLatin1());
        }
    }
    if (entries.size() > 256) {
        QMessageBox::warning(this, tr("Forced Colors"), tr("Enter up to 256 colors."));
        return;
    }
    customForced_ = colors;
    emit optionsChanged();
}

int IndexedColorDialog::palette() const { return palette_->currentData().toInt(); }

int IndexedColorDialog::colors() const
{
    switch (palette()) {
    case 0:
        return 256;
    case 1:
        return 216;
    default:
        return colors_->value();
    }
}

int IndexedColorDialog::dither() const { return dither_->currentData().toInt(); }

int IndexedColorDialog::amount() const { return amount_->value(); }

bool IndexedColorDialog::preview() const { return preview_->isChecked(); }

void IndexedColorDialog::setPreviewAvailable(bool available)
{
    preview_->setEnabled(available);
    if (!available) {
        preview_->setChecked(false);
    }
}

} // namespace pictura

#include "moc_indexed_color_dialog.cpp"
