#include "import_pdf_dialog.h"

#include "raster_size_fields.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h"

#include <QtCore/QFileInfo>
#include <QtCore/QSignalBlocker>
#include <QtCore/QtEndian>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

// Thumbnails are rendered once at this size and scaled for the grid.
constexpr int kThumbRender = 256;
constexpr int kThumbSmall = 64;
constexpr int kThumbLarge = 128;
constexpr int kDefaultCrop = 0; // Bounding Box

const char* const kCropTo[] = {"Bounding Box", "Media Box", "Crop Box",
                               "Bleed Box",    "Trim Box",  "Art Box"};

} // namespace

PdfImportProbe PdfImportProbe::parse(const QString& text)
{
    PdfImportProbe probe;
    for (const QString& line : text.split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
        const QStringList t = line.split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (t.value(0) == QStringLiteral("page") && t.size() == 26) {
            Page page;
            page.viewHeight = t[1].toDouble();
            for (int k = 0; k < 6; ++k) {
                const double x0 = t[2 + k * 4].toDouble();
                const double y0 = t[3 + k * 4].toDouble();
                const double x1 = t[4 + k * 4].toDouble();
                const double y1 = t[5 + k * 4].toDouble();
                page.boxes[k] = QRectF(QPointF(x0, y0), QPointF(x1, y1));
            }
            probe.pages.append(page);
        } else if (t.value(0) == QStringLiteral("images")) {
            probe.imagesAvailable = t.value(1).toInt() >= 0;
        } else if (t.value(0) == QStringLiteral("image") && t.size() == 4) {
            probe.images.append({t[1].toInt(), t[2].toInt(), t[3].toInt()});
        }
    }
    return probe;
}

QVector<QImage> unpackThumbnails(const QByteArray& packed)
{
    QVector<QImage> out;
    qsizetype at = 0;
    while (at + 8 <= packed.size()) {
        const quint32 w = qFromLittleEndian<quint32>(packed.constData() + at);
        const quint32 h = qFromLittleEndian<quint32>(packed.constData() + at + 4);
        const qsizetype bytes = qsizetype(w) * h * 4;
        at += 8;
        if (w == 0 || h == 0 || at + bytes > packed.size()) {
            break;
        }
        QImage image(reinterpret_cast<const uchar*>(packed.constData() + at), int(w), int(h),
                     int(w) * 4, QImage::Format_RGBA8888);
        out.append(image.copy());
        at += bytes;
    }
    return out;
}

namespace {

QByteArray toBytes(const ::rust::Vec<uint8_t>& v)
{
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

// A thumbnail on white, as a page reads on paper.
QIcon thumbnailIcon(const QImage& image)
{
    QImage paper(image.size(), QImage::Format_RGB32);
    paper.fill(Qt::white);
    QPainter painter(&paper);
    painter.drawImage(0, 0, image);
    painter.end();
    return QIcon(QPixmap::fromImage(paper));
}

} // namespace

ImportPdfDialog::ImportPdfDialog(const QString& path, const QString& password,
                                 const PdfImportProbe& probe, QWidget* parent)
    : QDialog(parent)
    , path_(path)
    , password_(password)
    , probe_(probe)
{
    setObjectName(QStringLiteral("importPdfDialog"));
    setWindowTitle(tr("Import PDF"));
    pageThumbs_ = unpackThumbnails(toBytes(pdf_import_thumbnails(path_, password_, kThumbRender)));
    buildUi();
    showPages();
}

void ImportPdfDialog::buildUi()
{
    auto* root = new QHBoxLayout(this);

    auto* select = new QGroupBox(tr("Select:"), this);
    auto* left = new QVBoxLayout(select);
    auto* radios = new QHBoxLayout;
    pages_ = new QRadioButton(tr("Pages"), select);
    pages_->setObjectName(QStringLiteral("pages"));
    images_ = new QRadioButton(tr("Images"), select);
    images_->setObjectName(QStringLiteral("images"));
    threeD_ = new QRadioButton(tr("3D"), select);
    threeD_->setObjectName(QStringLiteral("threeD"));
    // ponytail: no 3D engine; the option is shown as CS6 lays it out.
    threeD_->setEnabled(false);
    images_->setEnabled(probe_.imagesAvailable && !probe_.images.isEmpty());
    pages_->setChecked(true);
    radios->addWidget(pages_);
    radios->addWidget(images_);
    radios->addWidget(threeD_);
    radios->addStretch();
    left->addLayout(radios);

    grid_ = new QListWidget(select);
    grid_->setObjectName(QStringLiteral("thumbnails"));
    grid_->setViewMode(QListView::IconMode);
    grid_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    grid_->setMovement(QListView::Static);
    grid_->setResizeMode(QListView::Adjust);
    grid_->setWrapping(true);
    grid_->setMinimumSize(320, 300);
    left->addWidget(grid_, 1);

    selectedLabel_ = new QLabel(select);
    selectedLabel_->setObjectName(QStringLiteral("selectedLabel"));
    left->addWidget(selectedLabel_);
    auto* thumbRow = new QHBoxLayout;
    thumbRow->addWidget(new QLabel(tr("Thumbnail Size:"), select));
    thumbSize_ = new QComboBox(select);
    thumbSize_->setObjectName(QStringLiteral("thumbnailSize"));
    thumbSize_->addItems({tr("Small"), tr("Large"), tr("Fit Page")});
    thumbRow->addWidget(thumbSize_);
    thumbRow->addStretch();
    left->addLayout(thumbRow);
    root->addWidget(select, 1);

    auto* right = new QVBoxLayout;
    pageOptions_ = new QGroupBox(tr("Page Options"), this);
    auto* form = new QFormLayout(pageOptions_);
    name_ = new QLineEdit(QFileInfo(path_).fileName(), pageOptions_);
    name_->setObjectName(QStringLiteral("name"));
    form->addRow(tr("Name:"), name_);
    cropTo_ = new QComboBox(pageOptions_);
    cropTo_->setObjectName(QStringLiteral("cropTo"));
    for (const char* label : kCropTo) {
        cropTo_->addItem(tr(label));
    }
    cropTo_->setCurrentIndex(kDefaultCrop);
    form->addRow(tr("Crop To:"), cropTo_);
    antiAliased_ = new QCheckBox(tr("Anti-aliased"), pageOptions_);
    antiAliased_->setObjectName(QStringLiteral("antiAliased"));
    antiAliased_->setChecked(true);
    form->addRow(QString(), antiAliased_);
    right->addWidget(pageOptions_);

    size_ = new RasterSizeFields(true, this);
    right->addWidget(size_);
    suppress_ = new QCheckBox(tr("Suppress Warnings"), this);
    suppress_->setObjectName(QStringLiteral("suppressWarnings"));
    right->addWidget(suppress_);
    right->addStretch();
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    right->addWidget(buttons);
    root->addLayout(right);

    connect(pages_, &QRadioButton::toggled, this, [this](bool on) {
        if (on) {
            showPages();
        }
    });
    connect(images_, &QRadioButton::toggled, this, [this](bool on) {
        if (on) {
            showImages();
        }
    });
    connect(grid_, &QListWidget::itemSelectionChanged, this, [this]() { selectionChanged(); });
    connect(cropTo_, &QComboBox::currentIndexChanged, this, [this]() { selectionChanged(); });
    connect(thumbSize_, &QComboBox::currentIndexChanged, this,
            [this]() { applyThumbnailSize(); });
}

void ImportPdfDialog::showPages()
{
    const QSignalBlocker block(grid_);
    grid_->clear();
    for (int i = 0; i < probe_.pages.size(); ++i) {
        auto* item = new QListWidgetItem(QString::number(i + 1), grid_);
        if (i < pageThumbs_.size()) {
            item->setIcon(thumbnailIcon(pageThumbs_[i]));
        }
    }
    pageOptions_->setEnabled(true);
    size_->setSizeEnabled(true);
    if (grid_->count() > 0) {
        grid_->item(0)->setSelected(true);
        grid_->setCurrentRow(0);
    }
    applyThumbnailSize();
    selectionChanged();
}

void ImportPdfDialog::showImages()
{
    if (!imageThumbsLoaded_) {
        imageThumbs_ =
            unpackThumbnails(toBytes(pdf_import_image_thumbnails(path_, kThumbRender)));
        imageThumbsLoaded_ = true;
    }
    const QSignalBlocker block(grid_);
    grid_->clear();
    for (int i = 0; i < probe_.images.size(); ++i) {
        auto* item = new QListWidgetItem(QString::number(i + 1), grid_);
        if (i < imageThumbs_.size()) {
            item->setIcon(thumbnailIcon(imageThumbs_[i]));
        }
    }
    pageOptions_->setEnabled(false);
    size_->setSizeEnabled(false);
    if (grid_->count() > 0) {
        grid_->item(0)->setSelected(true);
        grid_->setCurrentRow(0);
    }
    applyThumbnailSize();
    selectionChanged();
}

void ImportPdfDialog::applyThumbnailSize()
{
    int side = kThumbSmall;
    if (thumbSize_->currentIndex() == 1) {
        side = kThumbLarge;
    } else if (thumbSize_->currentIndex() == 2) {
        // Before the dialog is first shown the viewport has no real width yet.
        const int width = std::max(grid_->viewport()->width(), grid_->minimumWidth());
        side = std::max(kThumbSmall, width - 2 * grid_->spacing() - 24);
    }
    grid_->setIconSize(QSize(side, side));
    grid_->setGridSize(QSize(side + 12, side + grid_->fontMetrics().height() + 12));
}

void ImportPdfDialog::resizeEvent(QResizeEvent* event)
{
    QDialog::resizeEvent(event);
    if (thumbSize_ && thumbSize_->currentIndex() == 2) {
        applyThumbnailSize();
    }
}

void ImportPdfDialog::selectionChanged()
{
    const int total = imagesMode() ? probe_.images.size() : probe_.pages.size();
    selectedLabel_->setText(imagesMode()
                                ? tr("%1 of %2 image(s) selected").arg(selection().size()).arg(total)
                                : tr("%1 of %2 page(s) selected").arg(selection().size()).arg(total));
    if (!imagesMode() && !selection().isEmpty()) {
        size_->setSourceSize(largestBox());
    }
}

bool ImportPdfDialog::imagesMode() const
{
    return images_->isChecked();
}

QList<int> ImportPdfDialog::selection() const
{
    QList<int> rows;
    for (const QListWidgetItem* item : grid_->selectedItems()) {
        rows.append(grid_->row(item));
    }
    std::sort(rows.begin(), rows.end());
    return rows;
}

QString ImportPdfDialog::name() const
{
    return name_->text();
}

bool ImportPdfDialog::antiAliased() const
{
    return antiAliased_->isChecked();
}

bool ImportPdfDialog::suppressWarnings() const
{
    return suppress_->isChecked();
}

QRectF ImportPdfDialog::boxOf(int page) const
{
    if (page < 0 || page >= probe_.pages.size()) {
        return {};
    }
    return probe_.pages[page].boxes[std::clamp(cropTo_->currentIndex(), 0, 5)];
}

QSizeF ImportPdfDialog::largestBox() const
{
    QSizeF largest;
    for (int page : selection()) {
        const QRectF box = boxOf(page);
        largest = largest.expandedTo(QSizeF(std::abs(box.width()), std::abs(box.height())));
    }
    return largest;
}

QSize ImportPdfDialog::pixelSizeFor(int page) const
{
    const QSizeF largest = largestBox();
    const QSize fields = size_->pixelSize();
    const QRectF box = boxOf(page);
    if (largest.isEmpty()) {
        return fields;
    }
    return {std::max(1, int(std::lround(std::abs(box.width()) * fields.width() / largest.width()))),
            std::max(1, int(std::lround(std::abs(box.height()) * fields.height()
                                        / largest.height())))};
}

QString ImportPdfDialog::areaFor(int page) const
{
    const QRectF box = boxOf(page).normalized();
    const double viewHeight = page >= 0 && page < probe_.pages.size()
                                  ? probe_.pages[page].viewHeight
                                  : 0.0;
    return QStringLiteral("%1 %2 %3 %4 %5")
        .arg(box.left(), 0, 'g', 10)
        .arg(box.top(), 0, 'g', 10)
        .arg(box.right(), 0, 'g', 10)
        .arg(box.bottom(), 0, 'g', 10)
        .arg(viewHeight, 0, 'g', 10);
}

void ImportPdfDialog::setFieldForTest(const QString& name, const QString& value)
{
    if (name == QStringLiteral("select")) {
        grid_->clearSelection();
        for (const QString& n : value.split(QLatin1Char(','), Qt::SkipEmptyParts)) {
            if (QListWidgetItem* item = grid_->item(n.trimmed().toInt() - 1)) {
                item->setSelected(true);
            }
        }
    } else if (name == QStringLiteral("images")) {
        (value == QStringLiteral("1") ? images_ : pages_)->setChecked(true);
    } else if (name == QStringLiteral("name")) {
        name_->setText(value);
    } else if (name == QStringLiteral("cropTo")) {
        cropTo_->setCurrentIndex(cropTo_->findText(value));
    } else if (name == QStringLiteral("thumbnailSize")) {
        thumbSize_->setCurrentIndex(thumbSize_->findText(value));
    } else if (name == QStringLiteral("antiAliased")) {
        antiAliased_->setChecked(value == QStringLiteral("1"));
    } else if (name == QStringLiteral("suppress")) {
        suppress_->setChecked(value == QStringLiteral("1"));
    } else {
        size_->setFieldForTest(name, value);
    }
}

QString ImportPdfDialog::fieldForTest(const QString& name) const
{
    const auto flag = [](bool on) { return on ? QStringLiteral("1") : QStringLiteral("0"); };
    if (name == QStringLiteral("selectedLabel")) {
        return selectedLabel_->text();
    }
    if (name == QStringLiteral("pagesChecked")) {
        return flag(pages_->isChecked());
    }
    if (name == QStringLiteral("imagesEnabled")) {
        return flag(images_->isEnabled());
    }
    if (name == QStringLiteral("threeDEnabled")) {
        return flag(threeD_->isEnabled());
    }
    if (name == QStringLiteral("iconSize")) {
        return QString::number(grid_->iconSize().width());
    }
    if (name == QStringLiteral("count")) {
        return QString::number(grid_->count());
    }
    if (name == QStringLiteral("name")) {
        return name_->text();
    }
    if (name == QStringLiteral("cropTo")) {
        return cropTo_->currentText();
    }
    if (name == QStringLiteral("thumbnailSize")) {
        return thumbSize_->currentText();
    }
    if (name == QStringLiteral("antiAliased")) {
        return flag(antiAliased_->isChecked());
    }
    if (name == QStringLiteral("suppress")) {
        return flag(suppress_->isChecked());
    }
    return size_->fieldForTest(name);
}

} // namespace pictura
