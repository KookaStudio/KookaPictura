#include "refine_edge_dialog.h"

#include "panels/jump_slider.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/refine.cxxqt.h"

#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kPreviewSize = 220;

// CS6's view modes and their stored index (the engine's ViewMode order).
const char* const kViewModes[] = {
    "Marching Ants", "Overlay",    "On Black",  "On White",
    "Black & White", "On Layers",  "Reveal Layer",
};

// Add a label + spin box + slider row; returns the spin box. The slider and
// spin stay in sync.
QSpinBox* addSliderRow(QVBoxLayout* layout, QWidget* parent, const QString& label,
                       const QString& name, int lo, int hi, int value, const QString& suffix,
                       QSlider** sliderOut) {
    auto* head = new QHBoxLayout;
    head->addWidget(new QLabel(label, parent));
    head->addStretch(1);
    auto* spin = new QSpinBox(parent);
    spin->setObjectName(name);
    spin->setRange(lo, hi);
    spin->setValue(value);
    spin->setSuffix(suffix);
    spin->setFixedWidth(64);
    head->addWidget(spin);
    layout->addLayout(head);
    auto* slider = new JumpSlider(Qt::Horizontal, parent);
    slider->setObjectName(name + QStringLiteral("Slider"));
    slider->setRange(lo, hi);
    slider->setValue(value);
    layout->addWidget(slider);
    QObject::connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
    QObject::connect(spin, &QSpinBox::valueChanged, slider, &QSlider::setValue);
    *sliderOut = slider;
    return spin;
}

} // namespace

RefineEdgeDialog::RefineEdgeDialog(PictureView* view, QWidget* parent)
    : QDialog(parent)
    , view_(view)
{
    setWindowTitle(QStringLiteral("Refine Edge"));
    setObjectName(QStringLiteral("refineEdgeDialog"));
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(12, 12, 12, 12);
    root->setSpacing(8);

    // View Mode.
    auto* viewGroup = new QGroupBox(QStringLiteral("View Mode"), this);
    auto* viewLayout = new QVBoxLayout(viewGroup);
    auto* modeRow = new QHBoxLayout;
    modeRow->addWidget(new QLabel(QStringLiteral("View:"), viewGroup));
    viewMode_ = new QComboBox(viewGroup);
    viewMode_->setObjectName(QStringLiteral("refineViewMode"));
    for (const char* name : kViewModes) {
        viewMode_->addItem(QString::fromLatin1(name));
    }
    modeRow->addWidget(viewMode_, 1);
    viewLayout->addLayout(modeRow);
    showRadius_ = new QCheckBox(QStringLiteral("Show Radius (J)"), viewGroup);
    showRadius_->setObjectName(QStringLiteral("refineShowRadius"));
    showOriginal_ = new QCheckBox(QStringLiteral("Show Original (P)"), viewGroup);
    showOriginal_->setObjectName(QStringLiteral("refineShowOriginal"));
    viewLayout->addWidget(showRadius_);
    viewLayout->addWidget(showOriginal_);
    root->addWidget(viewGroup);

    // Edge Detection.
    auto* detectGroup = new QGroupBox(QStringLiteral("Edge Detection"), this);
    auto* detectLayout = new QVBoxLayout(detectGroup);
    smartRadius_ = new QCheckBox(QStringLiteral("Smart Radius"), detectGroup);
    smartRadius_->setObjectName(QStringLiteral("refineSmartRadius"));
    detectLayout->addWidget(smartRadius_);
    radius_ = addSliderRow(detectLayout, detectGroup, QStringLiteral("Radius:"),
                           QStringLiteral("refineRadius"), 0, 100, 0, QStringLiteral(" px"),
                           &radiusSlider_);
    root->addWidget(detectGroup);

    // Adjust Edge.
    auto* adjustGroup = new QGroupBox(QStringLiteral("Adjust Edge"), this);
    auto* adjustLayout = new QVBoxLayout(adjustGroup);
    smooth_ = addSliderRow(adjustLayout, adjustGroup, QStringLiteral("Smooth:"),
                           QStringLiteral("refineSmooth"), 0, 100, 0, QString(), &smoothSlider_);
    feather_ = addSliderRow(adjustLayout, adjustGroup, QStringLiteral("Feather:"),
                            QStringLiteral("refineFeather"), 0, 100, 0, QStringLiteral(" px"),
                            &featherSlider_);
    contrast_ = addSliderRow(adjustLayout, adjustGroup, QStringLiteral("Contrast:"),
                             QStringLiteral("refineContrast"), 0, 100, 0, QStringLiteral(" %"),
                             &contrastSlider_);
    shift_ = addSliderRow(adjustLayout, adjustGroup, QStringLiteral("Shift Edge:"),
                          QStringLiteral("refineShiftEdge"), -100, 100, 0, QStringLiteral(" %"),
                          &shiftSlider_);
    root->addWidget(adjustGroup);

    // Output.
    auto* outputGroup = new QGroupBox(QStringLiteral("Output"), this);
    auto* outputLayout = new QVBoxLayout(outputGroup);
    decontaminate_ = new QCheckBox(QStringLiteral("Decontaminate Colors"), outputGroup);
    decontaminate_->setObjectName(QStringLiteral("refineDecontaminate"));
    outputLayout->addWidget(decontaminate_);
    amount_ = addSliderRow(outputLayout, outputGroup, QStringLiteral("Amount:"),
                           QStringLiteral("refineAmount"), 0, 100, 50, QStringLiteral(" %"),
                           &amountSlider_);
    auto* outRow = new QHBoxLayout;
    outRow->addWidget(new QLabel(QStringLiteral("Output To:"), outputGroup));
    output_ = new QComboBox(outputGroup);
    output_->setObjectName(QStringLiteral("refineOutput"));
    output_->addItem(QStringLiteral("Selection"));
    output_->addItem(QStringLiteral("Layer Mask"));
    output_->addItem(QStringLiteral("New Layer"));
    output_->addItem(QStringLiteral("New Layer with Layer Mask"));
    outRow->addWidget(output_, 1);
    outputLayout->addLayout(outRow);
    root->addWidget(outputGroup);

    previewLabel_ = new QLabel(this);
    previewLabel_->setObjectName(QStringLiteral("refinePreview"));
    previewLabel_->setAlignment(Qt::AlignCenter);
    previewLabel_->setMinimumSize(kPreviewSize, kPreviewSize / 2);
    previewLabel_->setFrameShape(QFrame::Box);
    root->addWidget(previewLabel_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    const auto onEdit = [this]() { refreshPreview(); };
    connect(radiusSlider_, &QSlider::valueChanged, this, onEdit);
    connect(smoothSlider_, &QSlider::valueChanged, this, onEdit);
    connect(featherSlider_, &QSlider::valueChanged, this, onEdit);
    connect(contrastSlider_, &QSlider::valueChanged, this, onEdit);
    connect(shiftSlider_, &QSlider::valueChanged, this, onEdit);
    connect(smartRadius_, &QCheckBox::toggled, this, onEdit);
    connect(viewMode_, &QComboBox::currentIndexChanged, this, onEdit);
    connect(showRadius_, &QCheckBox::toggled, this, onEdit);
    connect(showOriginal_, &QCheckBox::toggled, this, onEdit);
    connect(decontaminate_, &QCheckBox::toggled, this, [this]() { refreshEnabled(); });
    connect(amountSlider_, &QSlider::valueChanged, this, onEdit);
    connect(output_, &QComboBox::currentIndexChanged, this, [this]() { refreshEnabled(); });

    refreshEnabled();
    refreshPreview();
}

bool RefineEdgeDialog::smartRadius() const { return smartRadius_->isChecked(); }
int RefineEdgeDialog::radius() const { return radius_->value(); }
int RefineEdgeDialog::smooth() const { return smooth_->value(); }
int RefineEdgeDialog::feather() const { return feather_->value(); }
int RefineEdgeDialog::contrast() const { return contrast_->value(); }
int RefineEdgeDialog::shiftEdge() const { return shift_->value(); }
bool RefineEdgeDialog::decontaminate() const { return decontaminate_->isChecked(); }
int RefineEdgeDialog::amount() const { return amount_->value(); }
int RefineEdgeDialog::outputTo() const { return output_->currentIndex(); }

void RefineEdgeDialog::refreshEnabled()
{
    // Decontaminate changes pixel colour, so CS6 forbids the in-place targets.
    const bool decon = decontaminate_->isChecked();
    amount_->setEnabled(decon);
    amountSlider_->setEnabled(decon);
    QComboBox* output = output_;
    for (int i = 0; i <= 1; ++i) {
        // Disable the item by removing and re-inserting via the model flags.
        auto* model = qobject_cast<QStandardItemModel*>(output->model());
        if (model) {
            QStandardItem* item = model->item(i);
            if (item) {
                item->setFlags(decon ? item->flags() & ~Qt::ItemIsEnabled : item->flags() | Qt::ItemIsEnabled);
            }
        }
    }
    if (decon && output_->currentIndex() <= 1) {
        output_->setCurrentIndex(2);
    }
}

void RefineEdgeDialog::refreshPreview()
{
    if (!view_) {
        return;
    }
    int width = 0;
    int height = 0;
    const ::rust::Vec<uint8_t> coverage = refine_edge_preview(
        *view_, smartRadius(), radius(), smooth(), feather(), contrast(), shiftEdge(),
        kPreviewSize, width, height);
    if (width <= 0 || height <= 0 || int(coverage.size()) != width * height) {
        preview_ = QImage();
        previewLabel_->clear();
        return;
    }
    // The mask is shown as coverage over black (On Black / Black & White) or
    // white (On White); Overlay and the layer modes fall back to the mask.
    const int mode = viewMode_->currentIndex();
    const bool onWhite = mode == 3;
    const int background = onWhite ? 255 : 0;
    QImage image(width, height, QImage::Format_Grayscale8);
    for (int y = 0; y < height; ++y) {
        const uint8_t* row = coverage.data() + size_t(y) * size_t(width);
        uint8_t* out = image.scanLine(y);
        for (int x = 0; x < width; ++x) {
            const int c = row[x];
            out[x] = uint8_t(qBound(0, (c + background) / 2, 255));
        }
    }
    preview_ = image;
    previewLabel_->setPixmap(QPixmap::fromImage(image));
}

} // namespace pictura
