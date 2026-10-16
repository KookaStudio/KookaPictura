#include "layers_panel.h"

#include "api/layers_api.h"
#include "dialogs.h"

#include "layer_new_dialog.h"
#include "layer_style_dialog.h"
#include "layers_filter_bar.h"
#include "layers_filter_proxy.h"
#include "layers_panel_internal.h"

#include "icons.h"
#include "layer_adjustments.h"
#include "percent_field.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QEvent>
#include <QtCore/QHash>
#include <QtCore/QItemSelection>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QMimeData>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QSet>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtCore/QVector>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QFont>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPen>
#include <QtGui/QPixmap>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyledItemDelegate>
#include <QtWidgets/QStyleOptionViewItem>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <array>
#include <functional>
#include <memory>
#include <utility>

namespace pictura {

// CS6 sheet colors (PSD `lclr`), index 1..7; 0 is no label.
QColor layerLabelColor(int label)
{
    switch (label) {
    case 1:
        return QColor(255, 0, 0);
    case 2:
        return QColor(255, 153, 0);
    case 3:
        return QColor(255, 255, 0);
    case 4:
        return QColor(0, 204, 0);
    case 5:
        return QColor(0, 102, 255);
    case 6:
        return QColor(153, 0, 204);
    case 7:
        return QColor(153, 153, 153);
    default:
        return QColor();
    }
}

QString layerTooltip(const LayerRowNode& layer)
{
    return QStringLiteral("%1 (%2)").arg(layer.name, layer.kind);
}

QPixmap labelSwatch(int label)
{
    const QColor color = layerLabelColor(label);
    if (!color.isValid()) {
        return {};
    }
    QPixmap swatch(10, 10);
    swatch.fill(color);
    return swatch;
}

LayersPanel::LayersPanel(QWidget* parent)
    : QWidget(parent)
{
    QWidget* body = this;
    auto* layout = new QVBoxLayout(body);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);

    // Panel Options are session state (schema v3); clamp a corrupt store.
    const SessionState session = pictura::loadSession();
    thumbSizeIndex_ = qBound(0, session.layersThumbSize, 3);
    thumbContents_ = qBound(0, session.layersThumbContents, 1);
    expandNewEffects_ = session.layersExpandNewEffects;
    addCopyOnDuplicate_ = session.layersAddCopyOnDuplicate;
    useDefaultMasksOnFill_ = session.layersUseDefaultMasksOnFill;
    thumbEntireDocument_ = thumbContents_ == 0;

    filterBar_ = new LayerFilterBar(body);
    layout->addWidget(filterBar_);

    auto* controls = new QHBoxLayout();
    blend_ = new QComboBox(body);
    blend_->setObjectName(QStringLiteral("layersBlendCombo"));
    for (const BlendEntry& entry : kBlends) {
        blend_->addItem(QString::fromLatin1(entry.name), QString::fromLatin1(entry.key));
    }
    blend_->setEnabled(false);
    blend_->setSizeAdjustPolicy(QComboBox::AdjustToContents);
    controls->addWidget(blend_, 1);
    opacity_ = new PercentField(tr("Opacity"), body);
    opacity_->setObjectName(QStringLiteral("layersOpacityField"));
    opacity_->setLabelAlignment(Qt::AlignRight | Qt::AlignVCenter);
    opacity_->setEnabled(false);
    opacity_->setToolTip(tr("Opacity"));
    controls->addWidget(opacity_, 1);
    layout->addLayout(controls);

    // The lock strip sits directly above the layer list; Fill shares its row.
    // Each icon is one flag; "All" is the derived four-bit set. The panel
    // reflects state, so toggling an individual flag off unchecks "All".
    auto* locks = new QHBoxLayout();
    locks->addSpacing(4);
    auto* lockLabel = new QLabel(tr("Lock:"), body);
    locks->addWidget(lockLabel);
    const auto makeLock = [this, body, locks](const QString& assetId, const QString& tooltip,
                                              const QString& flag) {
        auto* button = new QToolButton(body);
        button->setCheckable(true);
        button->setEnabled(false);
        button->setIcon(pictura::icon(assetId));
        button->setIconSize(QSize(16, 16));
        button->setAutoRaise(true);
        button->setToolTip(tooltip);
        locks->addWidget(button);
        connect(button, &QToolButton::toggled, this, [this, flag](bool on) {
            if (syncing_ || !view_) {
                return;
            }
            const QStringList paths = selectedPaths();
            if (!paths.isEmpty()) {
                view_->set_layers_lock(paths, flag, on);
            }
        });
        return button;
    };
    lockTransparency_ = makeLock(QStringLiteral("layers.lockAlpha"),
                                 tr("Lock Transparent Pixels"), QStringLiteral("transparency"));
    lockPixels_ = makeLock(QStringLiteral("layers.lockPaint"), tr("Lock Image Pixels"),
                           QStringLiteral("pixels"));
    lockPosition_ = makeLock(QStringLiteral("layers.lockPosition"), tr("Lock Position"),
                             QStringLiteral("position"));
    lockNesting_ = makeLock(QStringLiteral("layers.lockNesting"), tr("Lock Nesting"),
                            QStringLiteral("nesting"));
    lockNesting_->setVisible(false);
    lockAll_ = makeLock(QStringLiteral("layers.lockAll"), tr("Lock All"),
                        QStringLiteral("all"));
    locks->addStretch(1);
    fill_ = new PercentField(tr("Fill"), body);
    fill_->setObjectName(QStringLiteral("layersFillField"));
    fill_->setEnabled(false);
    fill_->setToolTip(tr("Fill"));
    locks->addWidget(fill_);
    layout->addLayout(locks);

    model_ = new LayersModel(this);
    proxy_ = new LayersFilterProxyModel(model_, this);

    tree_ = new LayersTreeView(body);
    tree_->setModel(proxy_);
    delegate_ = new LayerRowDelegate(tree_);
    delegate_->setThumbnailSize(kThumbSizePx.at(thumbSizeIndex_));
    tree_->setItemDelegate(delegate_);
    // The delegate draws the nesting indent and group chevron itself, so the
    // eye stays anchored at the panel's left edge for every depth.
    tree_->setRootIsDecorated(false);
    tree_->setIndentation(0);
    tree_->setItemsExpandable(true);
    tree_->setExpandsOnDoubleClick(false);
    // The viewport filter starts a rename only inside the name rect; Qt's own
    // DoubleClicked trigger would edit the whole row, so it is disabled.
    tree_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    tree_->setAllColumnsShowFocus(true);
    tree_->setSelectionBehavior(QAbstractItemView::SelectRows);
    tree_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    // Folder rows are shorter than layer rows, so per-row heights differ.
    tree_->setUniformRowHeights(false);
    tree_->setHeaderHidden(true);
    tree_->header()->setSectionResizeMode(0, QHeaderView::Stretch);
    tree_->setContextMenuPolicy(Qt::CustomContextMenu);
    tree_->viewport()->installEventFilter(this);
    // Mouse tracking so the viewport filter can show the pointing-hand cursor on
    // a row.
    tree_->viewport()->setMouseTracking(true);
    // Self-contained layer drag/drop: the tree resolves the drop to a target
    // path + mode and the panel turns it into one reparent/move undo step.
    tree_->setDragPathsProvider([this] {
        const QString path = currentPath();
        if (path.isEmpty()) {
            return QStringList{};
        }
        const QModelIndex index = model_->indexForPath(path);
        if (index.isValid() && index.data(SyntheticRole).toBool()) {
            return QStringList{};
        }
        return QStringList{path};
    });
    tree_->setPathResolver([this](const QModelIndex& index) { return pathForProxyIndex(index); });
    tree_->setDropHandler([this](const QString& dragged, const QString& target, int mode) {
        return view_ && view_->move_layer_to(dragged, target, mode);
    });
    tree_->setDropValidator([this](const QString& dragged, const QString& target, int mode) {
        return view_ && view_->can_move_layer_to(dragged, target, mode);
    });
    layout->addWidget(tree_, 1);

    auto* buttons = new QHBoxLayout();
    buttons->setSpacing(0);
    const auto stripIconButton = [body, buttons](const QString& objectName,
                                                 const QString& assetId) {
        auto* button = new QToolButton(body);
        button->setObjectName(objectName);
        button->setIcon(pictura::icon(assetId));
        button->setIconSize(QSize(16, 16));
        button->setAutoRaise(true);
        buttons->addWidget(button);
        return button;
    };

    // CS6 strip order. Link has an icon but no behaviour yet, so it stays
    // disabled rather than pretending; fx opens Blending Options.
    auto* linkButton =
        stripIconButton(QStringLiteral("layersStripLink"), QStringLiteral("layers.link"));
    linkButton->setEnabled(false);
    linkButton->setToolTip(tr("Link Layers — not implemented yet"));
    auto* fxButton = stripIconButton(QStringLiteral("layersStripFx"), QStringLiteral("layers.fx"));
    fxButton->setToolTip(tr("Layer Style — click opens Blending Options; "
                            "Alt-click toggles all effects"));
    connect(fxButton, &QToolButton::clicked, this, [this] {
        if (!view_) {
            return;
        }
        if (QApplication::keyboardModifiers().testFlag(Qt::AltModifier)) {
            layer_style_set_all_visible(*view_, !layer_style_any_visible(*view_, true));
        } else {
            openLayerStyle(currentPath());
        }
    });
    maskButton_ =
        stripIconButton(QStringLiteral("layersStripMask"), QStringLiteral("layers.mask"));
    connect(maskButton_, &QToolButton::clicked, this, [this] { addMaskFromButton(); });

    auto* addButton = stripIconButton(QStringLiteral("layersStripFillAdjustment"),
                                      QStringLiteral("layers.fillAdjustment"));
    addButton->setPopupMode(QToolButton::InstantPopup);
    addButton->setToolTip(tr("New Fill / Adjustment Layer"));
    auto* menu = new QMenu(addButton);
    // ponytail: opaque black; the toolbox foreground swatch is C++-only and not
    // yet plumbed to the bridge. Swap in the live color when it is.
    QAction* solidFill = menu->addAction(tr("Solid Color…"));
    connect(solidFill, &QAction::triggered, this, [this] {
        if (view_) {
            view_->add_solid_fill(0xff000000u);
        }
    });
    QAction* gradientFill = menu->addAction(tr("Gradient…"));
    connect(gradientFill, &QAction::triggered, this, [this] {
        if (view_) {
            view_->add_gradient_fill();
        }
    });
    // ponytail: pattern authoring needs a pattern preset/library and a picker;
    // the engine renders a PtFl layer but has no encoder or pattern source.
    QAction* patternFill = menu->addAction(tr("Pattern…"));
    patternFill->setEnabled(false);
    patternFill->setToolTip(tr("Pattern Fill — not implemented yet"));
    menu->addSeparator();
    for (const LayerAdjustment& entry : kLayerAdjustments) {
        const QString leaf = QString::fromUtf8(entry.leaf);
        QAction* action = menu->addAction(entry.ellipsis ? leaf + QStringLiteral("…") : leaf);
        const QString kind = QString::fromLatin1(entry.kind);
        connect(action, &QAction::triggered, this, [this, kind] {
            if (view_) {
                view_->add_adjustment(kind);
            }
        });
    }
    addButton->setMenu(menu);

    auto* newGroupButton =
        stripIconButton(QStringLiteral("layersStripGroup"), QStringLiteral("layers.group"));
    newGroupButton->setToolTip(tr("New Group — drop a layer here to group it"));
    newGroupButton->setProperty("layerDropAction", QStringLiteral("group"));
    newGroupButton->setAcceptDrops(true);
    newGroupButton->installEventFilter(this);
    auto* newLayerButton =
        stripIconButton(QStringLiteral("layersStripNewLayer"), QStringLiteral("layers.newLayer"));
    newLayerButton->setToolTip(tr("New Layer — drop a layer here to duplicate it"));
    newLayerButton->setProperty("layerDropAction", QStringLiteral("duplicate"));
    newLayerButton->setAcceptDrops(true);
    newLayerButton->installEventFilter(this);
    deleteButton_ =
        stripIconButton(QStringLiteral("layersStripDelete"), QStringLiteral("layers.delete"));
    deleteButton_->setToolTip(tr("Delete — drop a layer here to delete it"));
    deleteButton_->setProperty("layerDropAction", QStringLiteral("delete"));
    deleteButton_->setAcceptDrops(true);
    deleteButton_->installEventFilter(this);
    layout->addLayout(buttons);

    connect(tree_->selectionModel(), &QItemSelectionModel::currentChanged, this,
            [this](const QModelIndex&, const QModelIndex&) {
                syncControls();
                syncMaskSolo();
            });
    connect(tree_->selectionModel(), &QItemSelectionModel::selectionChanged, this,
            [this](const QItemSelection&, const QItemSelection&) { syncControls(); });
    connect(tree_, &QTreeView::expanded, this, [this](const QModelIndex& index) {
        const QString path = pathForProxyIndex(index);
        if (!path.isEmpty()) {
            expandedPaths_.insert(path);
        }
    });
    connect(tree_, &QTreeView::collapsed, this, [this](const QModelIndex& index) {
        expandedPaths_.remove(pathForProxyIndex(index));
    });
    connect(filterBar_, &LayerFilterBar::filterChanged, this, &LayersPanel::applyFilter);
    connect(blend_, &QComboBox::currentIndexChanged, this, [this](int index) {
        if (syncing_ || !view_) {
            return;
        }
        const QStringList paths = selectedPaths();
        if (!paths.isEmpty()) {
            view_->set_layers_blend(paths, blend_->itemData(index).toString());
        }
    });
    connect(opacity_, &PercentField::valueChanged, this, [this](double pct) {
        if (syncing_ || !view_) {
            return;
        }
        const QStringList paths = selectedPaths();
        if (!paths.isEmpty()) {
            view_->preview_layers_opacity(paths, qRound(pct * 255.0 / 100.0));
        }
    });
    connect(opacity_, &PercentField::valueCommitted, this, [this](double pct) {
        if (syncing_ || !view_) {
            return;
        }
        const QStringList paths = selectedPaths();
        if (!paths.isEmpty()) {
            view_->commit_layers_opacity(paths, qRound(pct * 255.0 / 100.0));
        }
    });
    connect(fill_, &PercentField::valueChanged, this, [this](double pct) {
        if (syncing_ || !view_) {
            return;
        }
        const QStringList paths = selectedPaths();
        if (!paths.isEmpty()) {
            view_->preview_layers_fill(paths, qRound(pct * 255.0 / 100.0));
        }
    });
    connect(fill_, &PercentField::valueCommitted, this, [this](double pct) {
        if (syncing_ || !view_) {
            return;
        }
        const QStringList paths = selectedPaths();
        if (!paths.isEmpty()) {
            view_->commit_layers_fill(paths, qRound(pct * 255.0 / 100.0));
        }
    });
    connect(tree_, &QTreeView::customContextMenuRequested, this,
            &LayersPanel::showContextMenu);
    connect(newGroupButton, &QToolButton::clicked, this, [this] {
        if (QApplication::keyboardModifiers().testFlag(Qt::AltModifier)) {
            openNewGroupDialog();
        } else {
            addGroupAt(currentPath());
        }
    });
    connect(newLayerButton, &QToolButton::clicked, this, [this] {
        if (QApplication::keyboardModifiers().testFlag(Qt::AltModifier)) {
            openNewLayerDialog();
        } else {
            addLayerAt(currentPath());
        }
    });
    connect(deleteButton_, &QToolButton::clicked, this, [this] { deleteSelection(); });
}

void LayersPanel::setView(PictureView* view)
{
    if (viewConnection_) {
        QObject::disconnect(viewConnection_);
    }
    // A different view means a document switch: solo paths and the filter are
    // transient, so both reset.
    if (view_ != view) {
        clearSolo();
        activeThumbByPath_.clear();
        isolatedPaths_.clear();
        const LayerFilter defaultFilter{true};
        if (filterBar_) {
            filterBar_->setFilter(defaultFilter);
        }
        if (proxy_) {
            proxy_->setFilter(defaultFilter);
        }
    }
    view_ = view;
    model_->setView(view);
    tree_->setDragSource(view);
    if (view_) {
        set_layers_panel_options(*view_, addCopyOnDuplicate_, useDefaultMasksOnFill_);
        viewConnection_ = connect(view_, &PictureView::changed, this, [this] {
            QTimer::singleShot(0, this, [this] { refresh(); });
        });
    }
    refresh();
}

void LayersPanel::refresh()
{
    const QStringList selected = selectedPaths();
    const QString selectedPath = currentPath();
    // The model reset below clears the tree's current index, whose
    // `selectionChanged` syncs an empty active layer; remember the document's
    // active layer now so the fallback can restore it afterwards.
    const QString activePath = view_ ? view_->active_layer_path() : QString();
    // A Tab rename commits through `changed`; the queued refresh must keep the
    // next row's editor alive rather than dropping it on the model reset.
    const bool wasEditing = tree_->viewport()->findChild<QLineEdit*>() != nullptr;
    const int thumbSize = delegate_ ? delegate_->thumbnailSize() : 24;

    QVector<LayerRowNode> rows;
    if (view_) {
        // One crossing per rebuild: the bridge fills every row field in one
        // pass, including the shape-aware thumbnails and the effect names.
        const pictura::layers_api::Snapshot snap =
            pictura::layers_api::snapshot(*view_, thumbSize, thumbEntireDocument_);
        rows.reserve(snap.rows.size());
        // The mask edit target is one per view; the row whose path matches it
        // draws the active-thumbnail border. Reading it also clears a target
        // whose layer or mask is gone.
        const QString maskTargetPath = snap.maskTargetPath;
        for (const pictura::layers_api::LayerRow& source : snap.rows) {
            LayerRowNode row;
            row.path = source.path;
            row.depth = source.depth;
            row.name = source.name;
            row.kind = source.kind;
            row.visible = source.visible;
            row.blend = source.blend;
            row.opacity = source.opacity;
            row.fill = source.fill;
            row.lockBits = source.lockBits;
            row.color = source.color;
            row.clipping = source.clipping;
            row.hasMask = source.hasMask;
            row.hasAdjustment = source.hasAdjustment;
            row.expandable = source.expandable;
            row.childCount = source.childCount;
            row.linked = source.linked;
            row.placed = source.placed;
            row.documentWidth = snap.documentWidth;
            row.documentHeight = snap.documentHeight;
            row.shape = source.shape;
            row.smartObject = source.smartObject;
            row.hasStyle = source.hasStyle;
            row.hasBlendIf = source.hasBlendIf;
            row.styleEffects = source.styleEffects;
            // The raster mask target is authoritative for the active thumbnail;
            // a stale raster target on a non-target row falls back to the image.
            int activeThumb = activeThumbByPath_.value(row.path, 0);
            if (row.path == maskTargetPath) {
                activeThumb = 1;
            } else if (activeThumb == 1) {
                activeThumb = 0;
            }
            row.activeThumb = activeThumb;
            row.thumbnail = source.thumbnail;
            row.maskThumbnail = source.maskThumbnail;
            row.maskLinked = source.maskLinked;
            row.maskDisabled = source.maskDisabled;
            row.maskTarget = row.hasMask && row.path == maskTargetPath;
            row.hasVectorMask = source.hasVectorMask;
            row.vectorMaskThumbnail = source.vectorMaskThumbnail;
            row.vectorMaskLinked = source.vectorMaskLinked;
            row.vectorMaskDisabled = source.vectorMaskDisabled;
            rows.push_back(std::move(row));
        }
        // Display-only Smart Filters rows: a group header under each filtered
        // smart object plus one row per filter. The header carries the filter
        // count and the group eye; the children carry their own eyes.
        QVector<LayerRowNode> synthetic;
        for (int k = 0; k < rows.size(); ++k) {
            LayerRowNode& row = rows[k];
            if (row.path.isEmpty() || !layer_has_smart_filters(*view_, row.path)) {
                continue;
            }
            const int count = layer_smart_filter_count(*view_, row.path);
            // The smart object row gains a disclosure so the filter group can be
            // revealed, as CS6 nests "Smart Filters" under the layer.
            row.expandable = true;
            LayerRowNode group;
            group.path = row.path + QStringLiteral("/@sf");
            group.depth = row.depth + 1;
            group.name = QStringLiteral("Smart Filters");
            group.kind = QStringLiteral("smart-filters");
            group.visible = layer_smart_filters_enabled(*view_, row.path);
            group.expandable = true;
            group.childCount = count;
            group.synthetic = true;
            synthetic.push_back(std::move(group));
            for (int j = 0; j < count; ++j) {
                LayerRowNode child;
                child.path = row.path + QStringLiteral("/@sf/") + QString::number(j);
                child.depth = row.depth + 2;
                child.name = layer_smart_filter_name(*view_, row.path, j);
                child.kind = QStringLiteral("smart-filter");
                child.visible = layer_smart_filter_visible(*view_, row.path, j);
                child.synthetic = true;
                synthetic.push_back(std::move(child));
            }
        }
        for (LayerRowNode& row : synthetic) {
            rows.push_back(std::move(row));
        }
    }
    // Capture each expanded group's name chain before the model reset, so a
    // group whose index path shifts is re-found by name and kept expanded.
    const QSet<QString> previousExpansion = expandedPaths_;
    QHash<QString, QString> previousNames;
    for (const QString& path : previousExpansion) {
        previousNames.insert(path, model_->namePathForPath(path));
    }
    model_->setRows(std::move(rows));

    QSet<QString> live;
    for (const QString& path : previousExpansion) {
        const QString expectedName = previousNames.value(path);
        QModelIndex modelIndex = model_->indexForPath(path);
        // A shifted path may name a different layer; fall back to the name chain.
        if (modelIndex.isValid()
            && model_->namePathForPath(model_->pathForIndex(modelIndex)) != expectedName) {
            modelIndex = QModelIndex();
        }
        if (!modelIndex.isValid() && !expectedName.isEmpty()) {
            modelIndex = model_->indexForNamePath(expectedName);
        }
        if (!modelIndex.isValid()) {
            continue;
        }
        live.insert(model_->pathForIndex(modelIndex));
        const QModelIndex index = proxy_->mapFromSource(modelIndex);
        if (index.isValid()) {
            tree_->expand(index);
        }
    }
    expandedPaths_ = live;
    if (proxy_->filter().enabled && proxy_->hasActiveCriteria()) {
        expandMatchingGroups();
    }

    if (!selected.isEmpty()) {
        selectPaths(selected, selectedPath);
    }
    // A document switch or a model reset can leave the tree with no selected
    // row. Photoshop always has an active layer, so restore the document's
    // active layer (the topmost pixel layer on open), falling back to the
    // first displayed row only when the document has none.
    if (tree_->selectionModel()->selectedRows(0).isEmpty() && proxy_->rowCount() > 0) {
        const QModelIndex activeIndex =
            activePath.isEmpty() ? QModelIndex() : proxyIndexForPath(activePath);
        if (activeIndex.isValid()) {
            selectPaths(QStringList{activePath}, activePath);
        } else if (!tree_->currentIndex().isValid()) {
            tree_->setCurrentIndex(proxy_->index(0, 0));
        }
    }
    if (wasEditing) {
        const QModelIndex editIndex = proxyIndexForPath(selectedPath);
        if (editIndex.isValid()) {
            tree_->setCurrentIndex(editIndex);
            tree_->edit(editIndex);
        }
    }
    syncControls();
}

int LayersPanel::currentLayer() const
{
    const QString path = currentPath();
    if (path.isEmpty() || path.contains(QLatin1Char('/'))) {
        return -1;
    }
    bool ok = false;
    const int index = path.toInt(&ok);
    return ok ? index : -1;
}

QString LayersPanel::currentPath() const
{
    return pathForProxyIndex(tree_->currentIndex());
}

void LayersPanel::selectLayer(int index)
{
    if (index < 0) {
        return;
    }
    selectPath(QString::number(index));
}

void LayersPanel::selectPath(const QString& path)
{
    if (path.isEmpty()) {
        return;
    }
    const QModelIndex index = proxyIndexForPath(path);
    if (index.isValid()) {
        tree_->setCurrentIndex(index);
    }
}

void LayersPanel::selectPaths(const QStringList& paths, const QString& current)
{
    if (!model_ || !tree_ || !tree_->selectionModel()) {
        return;
    }
    if (paths.isEmpty()) {
        tree_->selectionModel()->clearSelection();
        return;
    }
    QItemSelection selection;
    QModelIndex first;
    QModelIndex currentIndex;
    for (const QString& path : paths) {
        const QModelIndex index = proxyIndexForPath(path);
        if (!index.isValid()) {
            continue;
        }
        selection.select(index, index);
        if (!first.isValid()) {
            first = index;
        }
        if (path == current) {
            currentIndex = index;
        }
    }
    if (selection.isEmpty()) {
        return;
    }
    tree_->selectionModel()->select(
        selection, QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
    if (!currentIndex.isValid()) {
        currentIndex = first;
    }
    tree_->selectionModel()->setCurrentIndex(currentIndex, QItemSelectionModel::NoUpdate);
    tree_->scrollTo(currentIndex);
}

QStringList LayersPanel::selectedPaths() const
{
    QStringList paths;
    if (!model_ || !tree_ || !tree_->selectionModel()) {
        return paths;
    }
    const QModelIndexList rows = tree_->selectionModel()->selectedRows(0);
    for (const QModelIndex& index : rows) {
        const QString path = pathForProxyIndex(index);
        if (!path.isEmpty()) {
            paths.push_back(path);
        }
    }
    return paths;
}

void LayersPanel::syncControls()
{
    const QModelIndex current = tree_->currentIndex();
    const QStringList paths = selectedPaths();
    if (view_) {
        const QString current = currentPath();
        view_->set_active_layer(paths.size() == 1 ? current : QString());
        // A mask edit target belongs to the active layer; selecting a different
        // layer drops it so the next paint edits the layer's pixels again. A
        // transient empty selection (a model reset) keeps it.
        if (!current.isEmpty() && mask_edit_target_get(*view_) != current) {
            mask_edit_target_set(*view_, QString());
        }
        // While the Selected filter isolates a set, scope Move Auto-Select to
        // it; otherwise leave Auto-Select unscoped.
        const bool isolated = proxy_ && proxy_->filter().enabled && proxy_->filter().selected;
        const QStringList scope(isolatedPaths_.begin(), isolatedPaths_.end());
        view_->isolation_set(scope, isolated);
    }
    const bool active = view_ && !paths.isEmpty();
    syncing_ = true;
    const std::array<QToolButton*, 5> lockButtons = {
        lockTransparency_, lockPixels_, lockPosition_, lockNesting_, lockAll_};
    for (QToolButton* button : lockButtons) {
        button->setChecked(false);
    }
    bool lockAny = false;
    bool blendAny = false;
    bool opacityAny = false;
    bool fillAny = false;
    bool forcedLocks = false;
    if (active) {
        // Show the current row's value; apply to the whole selection.
        const QModelIndex valueIndex =
            current.isValid() ? current : model_->indexForPath(paths.first());
        const int blend = blend_->findData(valueIndex.data(BlendRole).toString());
        blend_->setCurrentIndex(blend >= 0 ? blend : 0);
        opacity_->setValue(qRound(valueIndex.data(OpacityRole).toInt() * 100.0 / 255.0));
        fill_->setValue(qRound(valueIndex.data(FillRole).toInt() * 100.0 / 255.0));
        const int lockBits = valueIndex.data(LockRole).toInt();
        lockTransparency_->setChecked((lockBits & 0x01) != 0);
        lockPixels_->setChecked((lockBits & 0x02) != 0);
        lockPosition_->setChecked((lockBits & 0x04) != 0);
        lockNesting_->setChecked((lockBits & 0x08) != 0);
        lockAll_->setChecked((lockBits & 0x0F) == 0x0F);

        // Enable iff at least one selected node is eligible (frozen table).
        for (const QString& path : paths) {
            const QModelIndex index = model_->indexForPath(path);
            const QString kind = index.data(KindRole).toString();
            const bool background = kind == QLatin1String("background");
            const bool group = kind == QLatin1String("group");
            const bool fullLock = (index.data(LockRole).toInt() & 0x0F) == 0x0F;
            if (!background) {
                lockAny = true;
            }
            if (!background && !fullLock) {
                blendAny = true;
                opacityAny = true;
            }
            if (!background && !fullLock && !group) {
                fillAny = true;
            }
        }
        // A type or shape layer forces Transparency and Image on: show them
        // checked (the row projection already sets the bits) and refuse a click.
        forcedLocks = valueIndex.data(LayerRowShapeRole).toBool()
            || valueIndex.data(KindRole).toString() == QLatin1String("type");
    }
    // Set the final enabled state once: a control that stays enabled keeps its
    // keyboard focus, where a transient disable would drop it.
    blend_->setEnabled(blendAny);
    opacity_->setEnabled(opacityAny);
    fill_->setEnabled(fillAny);
    for (QToolButton* button : lockButtons) {
        button->setEnabled(lockAny);
    }
    if (forcedLocks) {
        lockTransparency_->setEnabled(false);
        lockPixels_->setEnabled(false);
    }
    syncing_ = false;
    syncEditTarget();
    syncStripButtons();
    emit selectionChanged();
}

void LayersPanel::setActiveThumb(const QString& path, int thumb)
{
    if (path.isEmpty()) {
        return;
    }
    activeThumbByPath_.insert(path, thumb);
    if (model_) {
        model_->setActiveThumb(path, thumb);
    }
}

void LayersPanel::openPanelOptions()
{
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Layers Panel Options"));
    auto* form = new QFormLayout(&dialog);
    auto* sizeBox = new QComboBox(&dialog);
    sizeBox->addItem(tr("None"));
    sizeBox->addItem(tr("Small"));
    sizeBox->addItem(tr("Medium"));
    sizeBox->addItem(tr("Large"));
    sizeBox->setCurrentIndex(qBound(0, thumbSizeIndex_, 3));
    auto* contentsBox = new QComboBox(&dialog);
    contentsBox->addItem(tr("Entire Document"));
    contentsBox->addItem(tr("Layer Bounds"));
    contentsBox->setCurrentIndex(qBound(0, thumbContents_, 1));
    auto* expandBox = new QCheckBox(tr("Expand New Effects"), &dialog);
    expandBox->setChecked(expandNewEffects_);
    auto* copyBox = new QCheckBox(tr("Add \"copy\" to Copied Layers and Groups"), &dialog);
    copyBox->setChecked(addCopyOnDuplicate_);
    auto* maskBox = new QCheckBox(tr("Use Default Masks on Fill Layers"), &dialog);
    maskBox->setChecked(useDefaultMasksOnFill_);
    form->addRow(tr("Thumbnail Size"), sizeBox);
    form->addRow(tr("Thumbnail Contents"), contentsBox);
    form->addRow(expandBox);
    form->addRow(copyBox);
    form->addRow(maskBox);
    auto* buttons =
        new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    form->addRow(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    thumbSizeIndex_ = sizeBox->currentIndex();
    thumbContents_ = contentsBox->currentIndex();
    expandNewEffects_ = expandBox->isChecked();
    addCopyOnDuplicate_ = copyBox->isChecked();
    useDefaultMasksOnFill_ = maskBox->isChecked();
    thumbEntireDocument_ = thumbContents_ == 0;
    if (delegate_) {
        delegate_->setThumbnailSize(kThumbSizePx.at(thumbSizeIndex_));
    }
    persistOptions();
    if (view_) {
        set_layers_panel_options(*view_, addCopyOnDuplicate_, useDefaultMasksOnFill_);
    }
    refresh();
}

void LayersPanel::persistOptions()
{
    SessionState state = pictura::loadSession();
    state.layersThumbSize = thumbSizeIndex_;
    state.layersThumbContents = thumbContents_;
    state.layersExpandNewEffects = expandNewEffects_;
    state.layersAddCopyOnDuplicate = addCopyOnDuplicate_;
    state.layersUseDefaultMasksOnFill = useDefaultMasksOnFill_;
    state.schemaVersion = 3;
    pictura::saveSession(state);
}

QModelIndex LayersPanel::proxyIndexForPath(const QString& path) const
{
    if (!proxy_ || !model_) {
        return {};
    }
    return proxy_->mapFromSource(model_->indexForPath(path));
}

QString LayersPanel::pathForProxyIndex(const QModelIndex& index) const
{
    if (!model_ || !index.isValid()) {
        return {};
    }
    return model_->pathForIndex(proxy_->mapToSource(index));
}

void LayersPanel::expandMatchingGroups()
{
    std::function<void(const QModelIndex&)> walk = [this, &walk](const QModelIndex& parent) {
        const int rows = model_->rowCount(parent);
        for (int row = 0; row < rows; ++row) {
            const QModelIndex child = model_->index(row, 0, parent);
            if (model_->rowCount(child) == 0) {
                continue;
            }
            if (proxy_->hasMatchingDescendant(child)) {
                const QModelIndex proxyIndex = proxyIndexForPath(child.data(PathRole).toString());
                if (proxyIndex.isValid()) {
                    tree_->expand(proxyIndex);
                }
            }
            walk(child);
        }
    };
    walk(QModelIndex());
}

void LayersPanel::applyFilter(const LayerFilter& filter)
{
    if (!proxy_) {
        return;
    }
    LayerFilter effective = filter;
    const bool wasIsolated = proxy_->filter().enabled && proxy_->filter().selected;
    const bool nowIsolated = effective.enabled && effective.selected;
    if (nowIsolated && !wasIsolated) {
        // A new isolation session: capture the current selection as the fixed
        // set. Re-entering after a release or a toggle-off re-captures rather
        // than resuming a stale or empty set.
        const QStringList paths = selectedPaths();
        isolatedPaths_ = QSet<QString>(paths.begin(), paths.end());
    }
    effective.selectedPaths = isolatedPaths_;
    proxy_->setFilter(effective);
    if (effective.enabled && proxy_->hasActiveCriteria()) {
        expandMatchingGroups();
    } else {
        for (const QString& path : expandedPaths_) {
            const QModelIndex index = proxyIndexForPath(path);
            if (index.isValid()) {
                tree_->expand(index);
            }
        }
    }
    syncControls();
}

void LayersPanel::enterIsolation()
{
    const QStringList paths = selectedPaths();
    // Isolating nothing would blank the panel; leave the filter as it is.
    if (paths.isEmpty()) {
        return;
    }
    isolatedPaths_ = QSet<QString>(paths.begin(), paths.end());
    if (filterBar_) {
        filterBar_->activateSelected();
    }
}

void LayersPanel::releaseIsolation()
{
    isolatedPaths_.clear();
    if (filterBar_) {
        filterBar_->setFilterEnabled(false);
    }
}

bool LayersPanel::isolationActive() const
{
    return proxy_ && proxy_->filter().enabled && proxy_->filter().selected;
}

void LayersPanel::releaseIsolationFor(const QString& path)
{
    isolatedPaths_.remove(path);
    // Keep the panel selection within the reduced set (in its existing order,
    // so the new current row is deterministic), then re-filter.
    QStringList remaining;
    for (const QString& selected : selectedPaths()) {
        if (isolatedPaths_.contains(selected)) {
            remaining.push_back(selected);
        }
    }
    if (remaining.isEmpty()) {
        releaseIsolation();
        return;
    }
    selectPaths(remaining, remaining.first());
    if (proxy_) {
        applyFilter(proxy_->filter());
    }
}

} // namespace pictura
