#include "indexed_color_dialog.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

IndexedColorDialog::IndexedColorDialog(bool exactAvailable, QWidget* parent) : QDialog(parent)
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
    palette_->setCurrentIndex(exactAvailable ? 0 : palette_->findData(2));
    paletteForm->addRow(tr("Palette:"), palette_);
    colors_ = new QSpinBox(paletteBox);
    colors_->setObjectName(QStringLiteral("indexedColors"));
    colors_->setRange(2, 256);
    colors_->setValue(256);
    paletteForm->addRow(tr("Colors:"), colors_);
    left->addWidget(paletteBox);

    auto* optionsBox = new QGroupBox(tr("Options"), this);
    auto* optionsForm = new QFormLayout(optionsBox);
    dither_ = new QComboBox(optionsBox);
    dither_->setObjectName(QStringLiteral("indexedDither"));
    dither_->addItem(tr("None"), 0);
    dither_->addItem(tr("Diffusion"), 1);
    dither_->addItem(tr("Pattern"), 2);
    dither_->addItem(tr("Noise"), 3);
    dither_->setCurrentIndex(1);
    optionsForm->addRow(tr("Dither:"), dither_);
    amount_ = new QSpinBox(optionsBox);
    amount_->setObjectName(QStringLiteral("indexedAmount"));
    amount_->setRange(0, 100);
    amount_->setValue(75);
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
    preview_->setChecked(true);
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

} // namespace pictura

#include "moc_indexed_color_dialog.cpp"
