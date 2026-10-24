#include "filter_gallery_dialog.h"
#include "dialogs.h"
#include "filter_commit.h"
#include "filter_param_controls.h"
#include "icons.h"

#include <QtCore/QByteArray>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QVector>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QShowEvent>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"

namespace pictura {

namespace {

const QSize kThumbnailSize(80, 56);

// CS6's zoom presets: percentage levels with separators, then the three named
// views. A positive value is a zoom factor; a negative one is a named view
// (Actual Pixels / Fit in View / Fit on Screen).
struct ZoomPreset {
    const char* text;
    double zoom;
};
const ZoomPreset kZoomPresets[] = {
    {"6%", 0.0625},
    {"12%", 0.125},
    {"25%", 0.25},
    {"33%", 1.0 / 3.0},
    {"50%", 0.5},
    {"66%", 2.0 / 3.0},
    {"", 0.0},
    {"100%", 1.0},
    {"", 0.0},
    {"200%", 2.0},
    {"300%", 3.0},
    {"400%", 4.0},
    {"600%", 6.0},
    {"800%", 8.0},
    {"1000%", 10.0},
    {"1200%", 12.0},
    {"1600%", 16.0},
    {"", 0.0},
    {"Actual Pixels", -1.0},
    {"Fit in View", -2.0},
    {"Fit on Screen", -3.0},
};
// The step ladder is the preset percentage factors, ascending; the named views
// resolve through fitZoom(). Derived from the presets so the +/- buttons and the
// wheel can reach every level the drop-down lists.
QVector<double> zoomLadder()
{
    QVector<double> steps;
    for (const ZoomPreset& preset : kZoomPresets) {
        if (preset.zoom > 0.0) {
            steps.append(preset.zoom);
        }
    }
    return steps;
}
// Hold the preview until the controls settle.
const int kPreviewDelayMs = 120;
// Screen pixels filtered past each edge of the pane, so a filter that reads
// its neighbourhood sees real pixels at the visible border.
const int kPreviewMargin = 16;
// The thumbnails' source: the picture with its shorter side reduced to this.
const int kThumbnailSourceSide = 160;

// The gallery's membership in CS6: every Artistic filter, every Brush Strokes
// filter, three Distort, every Sketch filter, Glowing Edges, every Texture
// filter.
const char* const kGallery[][2] = {
    {"Artistic", "Colored Pencil"},  {"Artistic", "Cutout"},
    {"Artistic", "Dry Brush"},       {"Artistic", "Film Grain"},
    {"Artistic", "Fresco"},          {"Artistic", "Neon Glow"},
    {"Artistic", "Paint Daubs"},     {"Artistic", "Palette Knife"},
    {"Artistic", "Plastic Wrap"},    {"Artistic", "Poster Edges"},
    {"Artistic", "Rough Pastels"},   {"Artistic", "Smudge Stick"},
    {"Artistic", "Sponge"},          {"Artistic", "Underpainting"},
    {"Artistic", "Watercolor"},      {"Brush Strokes", "Accented Edges"},
    {"Brush Strokes", "Angled Strokes"}, {"Brush Strokes", "Crosshatch"},
    {"Brush Strokes", "Dark Strokes"},   {"Brush Strokes", "Ink Outlines"},
    {"Brush Strokes", "Spatter"},        {"Brush Strokes", "Sprayed Strokes"},
    {"Brush Strokes", "Sumi-e"},     {"Distort", "Diffuse Glow"},
    {"Distort", "Glass"},            {"Distort", "Ocean Ripple"},
    {"Sketch", "Bas Relief"},        {"Sketch", "Chalk & Charcoal"},
    {"Sketch", "Charcoal"},          {"Sketch", "Chrome"},
    {"Sketch", "Conté Crayon"},      {"Sketch", "Graphic Pen"},
    {"Sketch", "Halftone Pattern"},  {"Sketch", "Note Paper"},
    {"Sketch", "Photocopy"},         {"Sketch", "Plaster"},
    {"Sketch", "Reticulation"},      {"Sketch", "Stamp"},
    {"Sketch", "Torn Edges"},        {"Sketch", "Water Paper"},
    {"Stylize", "Glowing Edges"},    {"Texture", "Craquelure"},
    {"Texture", "Grain"},            {"Texture", "Mosaic Tiles"},
    {"Texture", "Patchwork"},        {"Texture", "Stained Glass"},
    {"Texture", "Texturizer"},
};

const FilterCommandSpec* specFor(const QString& kind)
{
    return filterCommandForKind(kind);
}

// The session's last stack: CS6 reopens the gallery on it.
QList<FilterGalleryDialog::Effect>& lastStack()
{
    static QList<FilterGalleryDialog::Effect> stack;
    return stack;
}

} // namespace

// The preview pane's content: the last rendered proxy, drawn over the part of
// the zoomed picture it covers.
class GalleryPreviewCanvas : public QWidget {
public:
    using QWidget::QWidget;

    QImage image;
    QRectF target;

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::SmoothPixmapTransform);
        painter.drawImage(target, image);
    }
};

QImage FilterGalleryDialog::thumbnailSample(const QImage& picture)
{
    if (picture.isNull()) {
        return {};
    }
    const QImage scaled = picture
                              .scaled(kThumbnailSize, Qt::KeepAspectRatioByExpanding,
                                      Qt::SmoothTransformation)
                              .convertToFormat(QImage::Format_RGBA8888);
    return scaled.copy((scaled.width() - kThumbnailSize.width()) / 2,
                       (scaled.height() - kThumbnailSize.height()) / 2, kThumbnailSize.width(),
                       kThumbnailSize.height());
}

QList<FilterGalleryDialog::Category> FilterGalleryDialog::categories()
{
    QList<Category> result;
    for (const auto& row : kGallery) {
        const QString family = QString::fromUtf8(row[0]);
        const FilterCommandSpec* spec = filterCommandForPath(
            {QStringLiteral("Filter"), family, QString::fromUtf8(row[1])});
        if (!spec || !filter_kind_supported(spec->kind)) {
            continue;
        }
        if (result.isEmpty() || result.last().first != family) {
            result.append({family, {}});
        }
        result.last().second.append(spec);
    }
    return result;
}

FilterGalleryDialog::FilterGalleryDialog(PictureView* view, QWidget* parent)
    : QDialog(parent), view_(view)
{
    setObjectName(QStringLiteral("filterGallery"));
    setWindowTitle(QStringLiteral("Filter Gallery"));
    if (view_) {
        // Sampled once, from a reduced copy; every filter's thumbnail renders
        // from the same bytes.
        const QSize doc = documentSize();
        thumbnailScale_ = doc.isEmpty()
                              ? 1.0
                              : std::max(double(kThumbnailSize.width()) / doc.width(),
                                         double(kThumbnailSize.height()) / doc.height());
        const double scale =
            std::min(1.0, double(kThumbnailSourceSide) / std::max(1, std::min(doc.width(), doc.height())));
        thumbnailSample_ = thumbnailSample(
            filter_gallery_preview(*view_, {}, {}, 0, 0, doc.width(), doc.height(), scale));
        for (int y = 0; y < thumbnailSample_.height(); ++y) {
            thumbnailRgba_.append(reinterpret_cast<const char*>(thumbnailSample_.constScanLine(y)),
                                  thumbnailSample_.width() * 4);
        }
    }

    previewTimer_ = new QTimer(this);
    previewTimer_->setSingleShot(true);
    previewTimer_->setInterval(kPreviewDelayMs);
    connect(previewTimer_, &QTimer::timeout, this, &FilterGalleryDialog::runPreview);

    auto* body = new QHBoxLayout(this);
    body->setSpacing(8);
    body->addWidget(buildPreview(), 1);
    thumbnailPane_ = buildThumbnails();
    body->addWidget(thumbnailPane_);

    auto* right = new QVBoxLayout;
    auto* topRow = new QHBoxLayout;
    auto* toggle = new QToolButton(this);
    toggle->setObjectName(QStringLiteral("galleryToggleThumbnails"));
    toggle->setToolTip(QStringLiteral("Show/hide the filter thumbnails"));
    toggle->setCheckable(true);
    toggle->setChecked(true);
    toggle->setAutoRaise(true);
    toggle->setIcon(icon(QStringLiteral("panel.rightClose")));
    connect(toggle, &QToolButton::toggled, thumbnailPane_, &QWidget::setVisible);
    connect(toggle, &QToolButton::toggled, toggle, [toggle](bool shown) {
        toggle->setIcon(icon(shown ? QStringLiteral("panel.rightClose")
                                   : QStringLiteral("panel.rightOpen")));
    });
    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    topRow->addWidget(toggle, 0, Qt::AlignTop);
    topRow->addLayout(buttons, 1);
    right->addLayout(topRow);

    filterCombo_ = new QComboBox(this);
    filterCombo_->setObjectName(QStringLiteral("galleryFilter"));
    QList<const FilterCommandSpec*> all;
    for (const Category& category : categories()) {
        all.append(category.second);
    }
    std::sort(all.begin(), all.end(), [](const FilterCommandSpec* a, const FilterCommandSpec* b) {
        return a->label.localeAwareCompare(b->label) < 0;
    });
    for (const FilterCommandSpec* spec : all) {
        filterCombo_->addItem(spec->label, spec->kind);
    }
    connect(filterCombo_, &QComboBox::activated, this,
            [this](int index) { selectFilter(filterCombo_->itemData(index).toString()); });
    right->addWidget(filterCombo_);

    options_ = new QWidget(this);
    optionsLayout_ = new QVBoxLayout(options_);
    optionsLayout_->setContentsMargins(0, 0, 0, 0);
    right->addWidget(options_);
    right->addStretch(1);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("galleryEffects"));
    list_->setDragDropMode(QAbstractItemView::InternalMove);
    list_->setMinimumHeight(140);
    list_->setIconSize(QSize(18, 18));
    // Visibility is each row's check state, so Space toggles it and assistive
    // technology reads it; the eye icon stands in for the hidden indicator.
    list_->viewport()->installEventFilter(this);
    connect(list_, &QListWidget::itemChanged, this, [this](QListWidgetItem* item) {
        const int index = effects_.size() - 1 - list_->row(item);
        const bool visible = item->checkState() == Qt::Checked;
        if (index >= 0 && index < effects_.size() && effects_.at(index).visible != visible) {
            setEffectVisible(index, visible);
        }
    });
    connect(list_, &QListWidget::currentRowChanged, this, [this](int row) {
        if (row >= 0) {
            selectEffect(effects_.size() - 1 - row);
        }
    });
    connect(list_->model(), &QAbstractItemModel::rowsMoved, this,
            [this](const QModelIndex&, int start, int, const QModelIndex&, int row) {
                const int to = row > start ? row - 1 : row;
                effectMoved(effects_.size() - 1 - start, effects_.size() - 1 - to);
            });
    right->addWidget(list_, 1);

    auto* listButtons = new QHBoxLayout;
    listButtons->addStretch(1);
    auto* add = new QToolButton(this);
    add->setObjectName(QStringLiteral("galleryNewEffect"));
    add->setIcon(icon(QStringLiteral("layers.newLayer")));
    add->setToolTip(QStringLiteral("New effect layer"));
    connect(add, &QToolButton::clicked, this, &FilterGalleryDialog::addEffect);
    auto* remove = new QToolButton(this);
    remove->setObjectName(QStringLiteral("galleryDeleteEffect"));
    remove->setIcon(icon(QStringLiteral("layers.delete")));
    remove->setToolTip(QStringLiteral("Delete effect layer"));
    connect(remove, &QToolButton::clicked, this, &FilterGalleryDialog::deleteEffect);
    listButtons->addWidget(add);
    listButtons->addWidget(remove);
    right->addLayout(listButtons);

    auto* rightPane = new QWidget(this);
    rightPane->setLayout(right);
    rightPane->setFixedWidth(260);
    body->addWidget(rightPane);

    effects_ = lastStack();
    if (effects_.isEmpty() && filterCombo_->count() > 0) {
        const QList<Category> groups = categories();
        effects_.append({groups.first().second.first()->kind, {}, true});
    }
    selected_ = effects_.size() - 1;
    rebuildList();
    selectEffect(selected_);

    resize(parent ? parent->size() * 0.9 : QSize(1200, 800));
    QTimer::singleShot(0, this, &FilterGalleryDialog::renderNextThumbnail);
}

QWidget* FilterGalleryDialog::buildPreview()
{
    auto* pane = new QWidget(this);
    auto* layout = new QVBoxLayout(pane);
    layout->setContentsMargins(0, 0, 0, 0);
    previewArea_ = new QScrollArea(pane);
    previewArea_->setObjectName(QStringLiteral("galleryPreview"));
    previewArea_->setAlignment(Qt::AlignCenter);
    previewArea_->setBackgroundRole(QPalette::Dark);
    previewCanvas_ = new GalleryPreviewCanvas;
    // Drags pan the viewport, so the canvas lets presses through to it.
    previewCanvas_->setAttribute(Qt::WA_TransparentForMouseEvents);
    previewArea_->setWidget(previewCanvas_);
    previewArea_->viewport()->installEventFilter(this);
    previewArea_->viewport()->setCursor(Qt::OpenHandCursor);
    for (QScrollBar* bar :
         {previewArea_->horizontalScrollBar(), previewArea_->verticalScrollBar()}) {
        connect(bar, &QScrollBar::valueChanged, this, &FilterGalleryDialog::schedulePreview);
    }
    layout->addWidget(previewArea_, 1);

    auto* zoomRow = new QHBoxLayout;
    auto* zoomOut = new QToolButton(pane);
    zoomOut->setObjectName(QStringLiteral("galleryZoomOut"));
    zoomOut->setText(QStringLiteral("−"));
    auto* zoomIn = new QToolButton(pane);
    zoomIn->setObjectName(QStringLiteral("galleryZoomIn"));
    zoomIn->setText(QStringLiteral("+"));
    connect(zoomOut, &QToolButton::clicked, this, [this] { stepZoom(false); });
    connect(zoomIn, &QToolButton::clicked, this, [this] { stepZoom(true); });
    addZoomShortcuts(this, zoomIn, zoomOut);

    // The zoom level: an editable box showing the live percentage, whose drop
    // down lists CS6's preset levels and the named views.
    zoomCombo_ = new QComboBox(pane);
    zoomCombo_->setObjectName(QStringLiteral("galleryZoom"));
    zoomCombo_->setEditable(true);
    zoomCombo_->setInsertPolicy(QComboBox::NoInsert);
    zoomCombo_->setMaxVisibleItems(24);
    zoomCombo_->setToolTip(QStringLiteral("Zoom level"));
    for (const ZoomPreset& preset : kZoomPresets) {
        if (preset.text[0] == '\0') {
            zoomCombo_->insertSeparator(zoomCombo_->count());
            continue;
        }
        zoomCombo_->addItem(QString::fromUtf8(preset.text), preset.zoom);
    }
    connect(zoomCombo_, &QComboBox::activated, this, [this](int index) {
        const double preset = zoomCombo_->itemData(index).toDouble();
        if (preset > 0.0) {
            setZoom(preset);
        } else if (preset == -1.0) {
            setZoom(1.0); // Actual Pixels
        } else {
            setZoom(fitZoom()); // Fit in View / Fit on Screen
        }
    });
    connect(zoomCombo_->lineEdit(), &QLineEdit::editingFinished, this, [this] {
        QString text = zoomCombo_->lineEdit()->text().trimmed();
        text.remove(QLatin1Char('%'));
        bool ok = false;
        const double percent = text.toDouble(&ok);
        if (ok && percent > 0.0) {
            setZoom(percent / 100.0);
        } else {
            layoutPreview(); // restore the shown value on bad input
        }
    });

    zoomRow->addWidget(zoomOut);
    zoomRow->addWidget(zoomIn);
    zoomRow->addWidget(zoomCombo_);
    zoomRow->addStretch(1);
    layout->addLayout(zoomRow);
    return pane;
}

QWidget* FilterGalleryDialog::buildThumbnails()
{
    auto* scroll = new QScrollArea(this);
    scroll->setObjectName(QStringLiteral("galleryThumbnails"));
    scroll->setWidgetResizable(true);
    auto* content = new QWidget;
    auto* column = new QVBoxLayout(content);
    column->setSpacing(2);
    bool first = true;
    for (const Category& category : categories()) {
        auto* header = new QToolButton(content);
        header->setObjectName(QStringLiteral("galleryCategory:") + category.first);
        header->setText(category.first);
        header->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        header->setAutoRaise(true);
        auto* grid = new QWidget(content);
        auto* gridLayout = new QGridLayout(grid);
        gridLayout->setContentsMargins(4, 0, 4, 4);
        int i = 0;
        for (const FilterCommandSpec* spec : category.second) {
            auto* thumb = new QToolButton(grid);
            thumb->setObjectName(QStringLiteral("galleryThumb:") + spec->kind);
            thumb->setProperty("kind", spec->kind);
            thumb->setText(spec->label);
            thumb->setToolButtonStyle(Qt::ToolButtonTextUnderIcon);
            thumb->setIconSize(kThumbnailSize);
            thumb->setCheckable(true);
            thumb->setAutoRaise(true);
            connect(thumb, &QToolButton::clicked, this,
                    [this, kind = spec->kind] { selectFilter(kind); });
            gridLayout->addWidget(thumb, i / 3, i % 3);
            thumbnails_.append(thumb);
            ++i;
        }
        const auto disclose = [header, grid](bool open) {
            grid->setVisible(open);
            header->setIcon(icon(open ? QStringLiteral("layers.disclosureDown")
                                      : QStringLiteral("layers.disclosureRight")));
        };
        connect(header, &QToolButton::clicked, grid,
                [disclose, grid] { disclose(grid->isHidden()); });
        disclose(first);
        first = false;
        column->addWidget(header);
        column->addWidget(grid);
    }
    column->addStretch(1);
    scroll->setWidget(content);
    scroll->setMinimumWidth(330);
    return scroll;
}

void FilterGalleryDialog::selectFilter(const QString& kind)
{
    if (selected_ < 0 || selected_ >= effects_.size() || !specFor(kind)) {
        return;
    }
    Effect& effect = effects_[selected_];
    if (effect.kind != kind) {
        effect.kind = kind;
        effect.values.clear();
    }
    rebuildList();
    selectEffect(selected_);
}

void FilterGalleryDialog::addEffect()
{
    if (selected_ < 0 || selected_ >= effects_.size()) {
        return;
    }
    effects_.insert(selected_ + 1, effects_.at(selected_));
    ++selected_;
    rebuildList();
    selectEffect(selected_);
}

void FilterGalleryDialog::deleteEffect()
{
    if (effects_.size() <= 1 || selected_ < 0 || selected_ >= effects_.size()) {
        return;
    }
    effects_.removeAt(selected_);
    selected_ = qMin(selected_, effects_.size() - 1);
    rebuildList();
    selectEffect(selected_);
}

void FilterGalleryDialog::setEffectVisible(int index, bool visible)
{
    if (index < 0 || index >= effects_.size()) {
        return;
    }
    effects_[index].visible = visible;
    showVisibility(list_->item(effects_.size() - 1 - index), visible);
    schedulePreview();
}

void FilterGalleryDialog::effectMoved(int from, int to)
{
    if (from == to || from < 0 || to < 0 || from >= effects_.size() || to >= effects_.size()) {
        return;
    }
    effects_.move(from, to);
    selected_ = to;
    // Rebuilt after the view finishes its own move.
    QTimer::singleShot(0, this, [this] {
        rebuildList();
        schedulePreview();
    });
}

void FilterGalleryDialog::rebuildList()
{
    const QSignalBlocker block(list_);
    list_->clear();
    // Top row = last applied, as CS6 stacks effect layers.
    for (int i = effects_.size() - 1; i >= 0; --i) {
        const FilterCommandSpec* spec = specFor(effects_.at(i).kind);
        auto* item = new QListWidgetItem(spec ? spec->label : effects_.at(i).kind, list_);
        item->setFlags(item->flags() | Qt::ItemIsDragEnabled | Qt::ItemIsUserCheckable);
        showVisibility(item, effects_.at(i).visible);
    }
    list_->setCurrentRow(effects_.size() - 1 - selected_);
}

void FilterGalleryDialog::showVisibility(QListWidgetItem* item, bool visible)
{
    if (!item) {
        return;
    }
    const QSignalBlocker block(list_);
    item->setCheckState(visible ? Qt::Checked : Qt::Unchecked);
    item->setIcon(icon(visible ? QStringLiteral("layers.eyeOn") : QStringLiteral("layers.eyeOff")));
}

void FilterGalleryDialog::selectEffect(int index)
{
    if (index < 0 || index >= effects_.size()) {
        return;
    }
    selected_ = index;
    Effect& effect = effects_[index];
    const FilterCommandSpec* spec = specFor(effect.kind);
    if (!spec) {
        return;
    }
    {
        const QSignalBlocker block(filterCombo_);
        filterCombo_->setCurrentIndex(filterCombo_->findData(effect.kind));
    }
    for (QToolButton* thumb : thumbnails_) {
        thumb->setChecked(thumb->property("kind").toString() == effect.kind);
    }
    if (controls_) {
        for (QWidget* row : controls_->rows()) {
            delete row;
        }
        delete controls_;
    }
    controls_ = new FilterParamControls(spec->params, options_);
    for (QWidget* row : controls_->rows()) {
        optionsLayout_->addWidget(row);
    }
    if (effect.values.isEmpty()) {
        effect.values = controls_->values();
    } else {
        controls_->setValues(effect.values);
    }
    connect(controls_, &FilterParamControls::changed, this, [this] {
        if (selected_ >= 0 && selected_ < effects_.size()) {
            effects_[selected_].values = controls_->values();
            schedulePreview();
        }
    });
    schedulePreview();
}

void FilterGalleryDialog::schedulePreview()
{
    previewTimer_->start();
}

void FilterGalleryDialog::runPreview()
{
    if (!view_ || !view_->has_document()) {
        return;
    }
    QStringList kinds;
    QList<double> params;
    for (const Effect& effect : effects_) {
        if (effect.visible) {
            kinds.append(effect.kind);
            params.append(effect.values);
        }
    }
    // Only what the pane shows is filtered, on a copy reduced to about one
    // pixel per device pixel; the document is untouched until OK filters the
    // layer at full resolution.
    const QSize doc = documentSize();
    const QSize viewport = previewArea_->viewport()->size();
    const double margin = kPreviewMargin / zoom_;
    const double left = previewArea_->horizontalScrollBar()->value() / zoom_ - margin;
    const double top = previewArea_->verticalScrollBar()->value() / zoom_ - margin;
    const int x0 = std::max(0, static_cast<int>(std::floor(left)));
    const int y0 = std::max(0, static_cast<int>(std::floor(top)));
    const int x1 = std::min(doc.width(),
                            static_cast<int>(std::ceil(left + viewport.width() / zoom_ + 2 * margin)));
    const int y1 = std::min(doc.height(),
                            static_cast<int>(std::ceil(top + viewport.height() / zoom_ + 2 * margin)));
    if (x1 <= x0 || y1 <= y0) {
        return;
    }
    const double scale = std::min(1.0, zoom_ * devicePixelRatioF());
    const QImage image =
        filter_gallery_preview(*view_, kinds, params, x0, y0, x1 - x0, y1 - y0, scale);
    if (image.isNull()) {
        return;
    }
    previewCanvas_->image = image;
    previewRegion_ = QRect(x0, y0, x1 - x0, y1 - y0);
    layoutPreview();
}

void FilterGalleryDialog::layoutPreview()
{
    const QSize doc = documentSize();
    previewCanvas_->resize(qMax(1, qRound(doc.width() * zoom_)), qMax(1, qRound(doc.height() * zoom_)));
    previewCanvas_->target = QRectF(previewRegion_.x() * zoom_, previewRegion_.y() * zoom_,
                                    previewRegion_.width() * zoom_, previewRegion_.height() * zoom_);
    previewCanvas_->update();
    if (zoomCombo_ && !zoomCombo_->lineEdit()->hasFocus()) {
        zoomCombo_->setEditText(QStringLiteral("%1%").arg(qRound(zoom_ * 1000.0) / 10.0));
    }
}

QSize FilterGalleryDialog::documentSize() const
{
    return view_ && view_->has_document() ? QSize(view_->document_width(), view_->document_height())
                                          : QSize();
}

QImage FilterGalleryDialog::previewImage() const
{
    return previewCanvas_->image;
}

double FilterGalleryDialog::fitZoom() const
{
    const QSize doc = documentSize();
    if (doc.isEmpty()) {
        return 1.0;
    }
    const QSize room = previewArea_->viewport()->size() - QSize(8, 8);
    const double fit = qMin(static_cast<double>(room.width()) / doc.width(),
                            static_cast<double>(room.height()) / doc.height());
    return qBound(zoomLadder().first(), fit, 1.0);
}

void FilterGalleryDialog::setZoom(double zoom)
{
    zoom_ = zoom;
    layoutPreview();
    schedulePreview();
}

// One step up or down CS6's zoom ladder.
void FilterGalleryDialog::stepZoom(bool zoomIn)
{
    const QVector<double> ladder = zoomLadder();
    if (zoomIn) {
        for (double step : ladder) {
            if (step > zoom_ + 1e-6) {
                setZoom(step);
                return;
            }
        }
        return;
    }
    double next = -1.0;
    for (double step : ladder) {
        if (step < zoom_ - 1e-6) {
            next = step;
        }
    }
    if (next > 0.0) {
        setZoom(next);
    }
}

void FilterGalleryDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // CS6 opens at 100 %, or smaller when the picture does not fit.
    setZoom(fitZoom());
}

bool FilterGalleryDialog::eventFilter(QObject* watched, QEvent* event)
{
    // The hand: a left drag on the preview scrolls it, as on the canvas.
    if (previewArea_ && watched == previewArea_->viewport()) {
        QWidget* viewport = previewArea_->viewport();
        QScrollBar* h = previewArea_->horizontalScrollBar();
        QScrollBar* v = previewArea_->verticalScrollBar();
        switch (event->type()) {
        case QEvent::MouseButtonPress: {
            auto* mouse = static_cast<QMouseEvent*>(event);
            if (mouse->button() == Qt::LeftButton) {
                panAnchor_ = mouse->position().toPoint();
                panStart_ = QPoint(h->value(), v->value());
                panning_ = true;
                viewport->setCursor(Qt::ClosedHandCursor);
                return true;
            }
            break;
        }
        case QEvent::MouseMove:
            if (panning_) {
                const QPoint delta =
                    static_cast<QMouseEvent*>(event)->position().toPoint() - panAnchor_;
                h->setValue(panStart_.x() - delta.x());
                v->setValue(panStart_.y() - delta.y());
                return true;
            }
            break;
        case QEvent::MouseButtonRelease:
            if (panning_ && static_cast<QMouseEvent*>(event)->button() == Qt::LeftButton) {
                panning_ = false;
                viewport->setCursor(Qt::OpenHandCursor);
                return true;
            }
            break;
        case QEvent::Wheel: {
            const int dy = static_cast<QWheelEvent*>(event)->angleDelta().y();
            if (dy != 0) {
                stepZoom(dy > 0);
                return true;
            }
            break;
        }
        default:
            break;
        }
    }
    // The eye: a press on a row's icon shows or hides that effect.
    if (list_ && watched == list_->viewport() && event->type() == QEvent::MouseButtonPress) {
        const QPoint pos = static_cast<QMouseEvent*>(event)->position().toPoint();
        const QListWidgetItem* item = list_->itemAt(pos);
        if (item && pos.x() < list_->visualItemRect(item).left() + list_->iconSize().width() + 8) {
            const int index = effects_.size() - 1 - list_->row(item);
            setEffectVisible(index, !effects_.at(index).visible);
            return true;
        }
    }
    return QDialog::eventFilter(watched, event);
}

void FilterGalleryDialog::renderNextThumbnail()
{
    if (nextThumbnail_ >= thumbnails_.size() || thumbnailSample_.isNull()) {
        return;
    }
    QToolButton* thumb = thumbnails_.at(nextThumbnail_++);
    const QImage filtered =
        filter_thumbnail(thumbnailRgba_, thumbnailSample_.width(), thumbnailSample_.height(),
                         thumb->property("kind").toString(), QList<double>(), thumbnailScale_);
    thumb->setIcon(QPixmap::fromImage(filtered.isNull() ? thumbnailSample_ : filtered));
    QTimer::singleShot(0, this, &FilterGalleryDialog::renderNextThumbnail);
}

FilterGalleryDialog::CommitResult FilterGalleryDialog::commit()
{
    previewTimer_->stop();
    QStringList kinds;
    QList<double> params;
    for (const Effect& effect : effects_) {
        if (effect.visible) {
            kinds.append(effect.kind);
            params.append(effect.values);
        }
    }
    lastStack() = effects_;
    if (kinds.isEmpty()) {
        return CommitResult::NothingVisible;
    }
    if (!view_) {
        return CommitResult::Refused;
    }
    QStringList names;
    for (const QString& kind : kinds) {
        const FilterCommandSpec* spec = filterCommandForKind(kind);
        names.append(spec ? spec->label : kind);
    }
    const QString label =
        QStringLiteral("Filter Gallery: %1").arg(names.join(QStringLiteral(", ")));
    PictureView* view = view_;
    const bool applied = runFilterCommit(parentWidget(), view, tr("Filter Gallery"), [&] {
        return filter_stack_commit_start(*view, kinds, params, label);
    });
    return applied ? CommitResult::Applied : CommitResult::Refused;
}

} // namespace pictura

#include "moc_filter_gallery_dialog.cpp"
