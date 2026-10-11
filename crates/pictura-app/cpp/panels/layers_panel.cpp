#include "layers_panel.h"
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
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

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

QString layerTooltip(const LayerRow& layer)
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
    for (const BlendEntry& entry : kBlends) {
        blend_->addItem(QString::fromLatin1(entry.name), QString::fromLatin1(entry.key));
    }
    blend_->setEnabled(false);
    blend_->setSizeAdjustPolicy(QComboBox::AdjustToContents);
    controls->addWidget(blend_, 1);
    opacity_ = new PercentField(tr("Opacity"), body);
    opacity_->setObjectName(QStringLiteral("layersOpacityField"));
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
    const auto stripIconButton = [body, buttons](const QString& objectName,
                                                 const QString& assetId) {
        auto* button = new QToolButton(body);
        button->setObjectName(objectName);
        button->setIcon(pictura::icon(assetId));
        button->setIconSize(QSize(20, 20));
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
    auto* maskButton =
        stripIconButton(QStringLiteral("layersStripMask"), QStringLiteral("layers.mask"));
    maskButton->setToolTip(tr("Add Layer Mask — click adds a mask for the selection "
                              "(or Reveal All); Alt-click adds Hide All"));
    connect(maskButton, &QToolButton::clicked, this, [this] {
        if (!view_) {
            return;
        }
        const QString kind =
            (QApplication::keyboardModifiers() & Qt::AltModifier) ? QStringLiteral("hide-all")
            : view_->has_selection() ? QStringLiteral("reveal-selection")
                                     : QStringLiteral("reveal-all");
        layer_mask_add(*view_, kind);
    });

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
    auto* deleteButton =
        stripIconButton(QStringLiteral("layersStripDelete"), QStringLiteral("layers.delete"));
    deleteButton->setToolTip(tr("Delete — drop a layer here to delete it"));
    deleteButton->setProperty("layerDropAction", QStringLiteral("delete"));
    deleteButton->setAcceptDrops(true);
    deleteButton->installEventFilter(this);
    layout->addLayout(buttons);

    connect(tree_->selectionModel(), &QItemSelectionModel::currentChanged, this,
            [this](const QModelIndex&, const QModelIndex&) { syncControls(); });
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
    connect(deleteButton, &QToolButton::clicked, this, [this] { deleteSelection(); });
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

    QVector<LayerRow> rows;
    if (view_) {
        const int count = view_->layer_row_count();
        const int documentWidth = view_->document_width();
        const int documentHeight = view_->document_height();
        rows.reserve(count);
        const QStringList styleEffectNames = layer_style_effect_names();
        // The mask edit target is one per view; the row whose path matches it
        // draws the active-thumbnail border. Reading it also clears a target
        // whose layer or mask is gone.
        const QString maskTargetPath = mask_edit_target_get(*view_);
        for (int i = 0; i < count; ++i) {
            LayerRow row;
            row.path = view_->layer_row_path(i);
            row.depth = view_->layer_row_depth(i);
            row.name = view_->layer_row_name(i);
            row.kind = view_->layer_row_kind(i);
            row.visible = view_->layer_row_visible(i);
            row.blend = view_->layer_row_blend(i);
            row.opacity = view_->layer_row_opacity(i);
            row.fill = view_->layer_row_fill(i);
            row.lockBits = view_->layer_row_lock(i);
            row.color = view_->layer_row_color(i);
            row.clipping = view_->layer_row_clipping(i);
            row.hasMask = view_->layer_row_has_mask(i);
            row.hasAdjustment = view_->layer_row_has_adjustment(i);
            row.expandable = view_->layer_row_expandable(i);
            row.childCount = view_->layer_row_child_count(i);
            row.linked = view_->layer_row_linked(i);
            row.placed = view_->layer_row_placed(i);
            row.documentWidth = documentWidth;
            row.documentHeight = documentHeight;
            row.shape = shape_row_is_shape(*view_, i);
            row.smartObject = layer_row_is_smart_object(*view_, i);
            row.hasStyle = layer_row_has_style(*view_, i);
            row.hasBlendIf = layer_row_has_blend_if(*view_, i);
            row.activeThumb = activeThumbByPath_.value(row.path, 0);
            if (row.hasStyle) {
                for (const QString& name : styleEffectNames) {
                    if (layer_style_value(*view_, row.path,
                                          name + QStringLiteral(".exists"))
                        == 1.0) {
                        row.styleEffects.push_back(name);
                    }
                }
            }
            row.thumbnail = row.shape
                ? shape_row_thumbnail(*view_, i, thumbSize)
                : view_->layer_row_thumbnail(i, thumbSize, thumbEntireDocument_);
            row.maskThumbnail = view_->layer_row_mask_thumbnail(i, thumbSize);
            row.maskLinked = layer_row_mask_linked(*view_, i);
            row.maskDisabled = layer_row_mask_disabled(*view_, i);
            row.maskTarget = row.hasMask && row.path == maskTargetPath;
            row.hasVectorMask = layer_row_has_vector_mask(*view_, i);
            row.vectorMaskThumbnail = layer_row_vector_mask_thumbnail(*view_, i, thumbSize);
            row.vectorMaskLinked = layer_row_vector_mask_linked(*view_, i);
            row.vectorMaskDisabled = layer_row_vector_mask_disabled(*view_, i);
            rows.push_back(std::move(row));
        }
        // Display-only Smart Filters rows: a group header under each filtered
        // smart object plus one row per filter. The header carries the filter
        // count and the group eye; the children carry their own eyes.
        QVector<LayerRow> synthetic;
        for (int k = 0; k < rows.size(); ++k) {
            LayerRow& row = rows[k];
            if (row.path.isEmpty() || !layer_has_smart_filters(*view_, row.path)) {
                continue;
            }
            const int count = layer_smart_filter_count(*view_, row.path);
            // The smart object row gains a disclosure so the filter group can be
            // revealed, as CS6 nests "Smart Filters" under the layer.
            row.expandable = true;
            LayerRow group;
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
                LayerRow child;
                child.path = row.path + QStringLiteral("/@sf/") + QString::number(j);
                child.depth = row.depth + 2;
                child.name = layer_smart_filter_name(*view_, row.path, j);
                child.kind = QStringLiteral("smart-filter");
                child.visible = layer_smart_filter_visible(*view_, row.path, j);
                child.synthetic = true;
                synthetic.push_back(std::move(child));
            }
        }
        for (LayerRow& row : synthetic) {
            rows.push_back(std::move(row));
        }
    }
    model_->setRows(std::move(rows));

    // Restore expansion by path; keep a path the filter hid (the document still
    // has it) so toggling the filter off restores its expansion. Iterate a copy:
    // `expand()` emits `expanded`, which re-inserts into the set.
    const QSet<QString> previousExpansion = expandedPaths_;
    QSet<QString> live;
    for (const QString& path : previousExpansion) {
        if (!model_->indexForPath(path).isValid()) {
            continue;
        }
        live.insert(path);
        const QModelIndex index = proxyIndexForPath(path);
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

bool LayersPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (auto* button = qobject_cast<QToolButton*>(watched)) {
        const QString dropAction = button->property("layerDropAction").toString();
        if (!dropAction.isEmpty()) {
            if (event->type() == QEvent::DragEnter) {
                auto* drag = static_cast<QDragEnterEvent*>(event);
                if (tree_->isOwnLayerDrag(drag->mimeData())) {
                    drag->acceptProposedAction();
                    return true;
                }
            } else if (event->type() == QEvent::Drop) {
                auto* drop = static_cast<QDropEvent*>(event);
                if (tree_->isOwnLayerDrag(drop->mimeData())) {
                    const QStringList paths =
                        QString::fromUtf8(drop->mimeData()->data(kLayerMimeType))
                            .split(QLatin1Char('\n'), Qt::SkipEmptyParts);
                    if (view_ && !paths.isEmpty()) {
                        if (dropAction == QLatin1String("delete")) {
                            view_->delete_layers(paths);
                        } else if (dropAction == QLatin1String("duplicate")) {
                            // A Background cannot be duplicated into a locked
                            // `Background copy`; convert it in place instead.
                            const QModelIndex row = model_->indexForPath(paths.first());
                            if (row.isValid()
                                && row.data(KindRole).toString()
                                    == QLatin1String("background")) {
                                openBackgroundConversion(paths.first());
                            } else {
                                view_->duplicate_layers(paths);
                            }
                        } else if (dropAction == QLatin1String("group")) {
                            view_->group_layers(paths);
                        }
                    }
                    drop->acceptProposedAction();
                    return true;
                }
            }
        }
    }
    if (watched == tree_->viewport()) {
        if (event->type() == QEvent::MouseMove || event->type() == QEvent::HoverMove) {
            auto* move = static_cast<QMouseEvent*>(event);
            const QModelIndex index = tree_->indexAt(move->position().toPoint());
            tree_->viewport()->setCursor(index.isValid() ? Qt::PointingHandCursor
                                                         : Qt::ArrowCursor);
        } else if (event->type() == QEvent::Leave) {
            tree_->viewport()->unsetCursor();
        }
    }
    if (watched == tree_->viewport() && event->type() == QEvent::MouseButtonPress) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            const QPoint pos = mouse->position().toPoint();
            const QModelIndex index = tree_->indexAt(pos);
            if (index.isValid() && index.data(ExpandableRole).toBool()) {
                const QRect chevron =
                    delegate_->chevronRect(tree_->visualRect(index), index);
                if (chevron.contains(pos)) {
                    tree_->setExpanded(index, !tree_->isExpanded(index));
                    return true;
                }
            }
            if (index.isValid() && (mouse->modifiers() & Qt::ControlModifier)) {
                const QRect thumb = delegate_->thumbRect(tree_->visualRect(index), index);
                if (!thumb.isEmpty() && thumb.contains(pos)) {
                    // Ctrl+click the thumbnail selects the layer's pixels;
                    // consume the click so it starts no drag and opens no editor.
                    if (view_) {
                        view_->select_layer_alpha(pathForProxyIndex(index));
                    }
                    return true;
                }
            }
            // The mask's own controls: Shift-click the mask thumbnail toggles
            // its enabled bit; a plain click makes it the mask edit target (the
            // brush, fill, and filters then write its coverage); a click on the
            // link glyph toggles its linkage. All are consumed so they never
            // rename, select, or start a drag.
            if (index.isValid() && view_) {
                const QString rowPath = pathForProxyIndex(index);
                const QRect vr = tree_->visualRect(index);
                // A plain layer-thumbnail click returns to editing the layer's
                // pixels; the click still selects the row or starts a drag.
                const QRect layerThumb = delegate_->thumbRect(vr, index);
                if (!layerThumb.isEmpty() && layerThumb.contains(pos)
                    && !(mouse->modifiers() & Qt::ControlModifier)) {
                    mask_edit_target_set(*view_, QString());
                }
                const QRect maskThumb = delegate_->maskThumbRect(vr, index);
                if (!maskThumb.isEmpty() && maskThumb.contains(pos)) {
                    setActiveThumb(rowPath, 1);
                    if (mouse->modifiers() & Qt::ShiftModifier) {
                        layer_mask_set_enabled_path(*view_, rowPath,
                                                    index.data(MaskDisabledRole).toBool());
                    } else {
                        // The target belongs to the active layer, so select the
                        // row first (its selection sets the active layer).
                        tree_->setCurrentIndex(index);
                        mask_edit_target_set(*view_, rowPath);
                        refresh();
                    }
                    return false;
                }
                const QRect linkGlyph = delegate_->linkGlyphRect(vr, index);
                if (!linkGlyph.isEmpty() && linkGlyph.contains(pos)) {
                    layer_mask_set_linked_path(*view_, rowPath,
                                               !index.data(MaskLinkedRole).toBool());
                    return true;
                }
                // The vector mask's own controls, mirroring the layer mask.
                const QRect vectorThumb = delegate_->vectorMaskThumbRect(vr, index);
                if (!vectorThumb.isEmpty() && vectorThumb.contains(pos)) {
                    setActiveThumb(rowPath, 2);
                    if (mouse->modifiers() & Qt::ShiftModifier) {
                        vector_mask_set_enabled_path(
                            *view_, rowPath, index.data(VectorMaskDisabledRole).toBool());
                        return true;
                    }
                    return false;
                }
                const QRect vectorLink = delegate_->vectorLinkGlyphRect(vr, index);
                if (!vectorLink.isEmpty() && vectorLink.contains(pos)) {
                    vector_mask_set_linked_path(*view_, rowPath,
                                                !index.data(VectorMaskLinkedRole).toBool());
                    return true;
                }
                // A plain or Alt click on the image thumbnail makes it the edit
                // target and still selects the row (which activates the layer).
                const QRect imageThumb = delegate_->thumbRect(vr, index);
                if (!imageThumb.isEmpty() && imageThumb.contains(pos)
                    && !(mouse->modifiers() & Qt::ControlModifier)) {
                    setActiveThumb(rowPath, 0);
                    return false;
                }
            }
            // An Alt-click on a row's fx badge toggles every layer's effects,
            // CS6's Show/Hide All Effects (a plain click leaves it to selection).
            if (index.isValid() && view_ && (mouse->modifiers() & Qt::AltModifier)) {
                const QRect fx = delegate_->fxRect(tree_->visualRect(index), index);
                if (!fx.isEmpty() && fx.contains(pos)) {
                    layer_style_set_all_visible(*view_,
                                                !layer_style_any_visible(*view_, true));
                    return true;
                }
            }
            // Alt-click on the line between two rows (outside the eye column)
            // clips the upper layer to the lower, or releases it, as CS6 does.
            if (index.isValid() && (mouse->modifiers() & Qt::AltModifier)
                && !delegate_->eyeColumnContains(tree_->visualRect(index), pos)) {
                const QRect row = tree_->visualRect(index);
                QModelIndex upper;
                if (pos.y() >= row.bottom() - kClipLineGrab) {
                    upper = index;
                } else if (pos.y() <= row.top() + kClipLineGrab) {
                    upper = tree_->indexAbove(index);
                }
                if (upper.isValid() && view_
                    && clipping_toggle(*view_, pathForProxyIndex(upper))) {
                    return true;
                }
            }
            // The Background's lock badge unlocks it: one click makes it an
            // ordinary layer, as Photoshop's lock icon does.
            if (index.isValid() && view_
                && index.data(KindRole).toString() == QLatin1String("background")
                && delegate_->lockRect(tree_->visualRect(index)).contains(pos)) {
                const QString path = pathForProxyIndex(index);
                if (view_->convert_background(path, view_->next_layer_name(QStringLiteral("Layer")),
                                              0)) {
                    refresh();
                    selectPath(path);
                }
                return true;
            }
            // A regular layer's lock badge unlocks it with one click; the
            // Background badge above converts instead.
            if (index.isValid() && view_ && index.data(LockRole).toInt() != 0
                && index.data(KindRole).toString() != QLatin1String("background")
                && delegate_->lockRect(tree_->visualRect(index)).contains(pos)) {
                view_->set_layers_lock(QStringList{pathForProxyIndex(index)},
                                       QStringLiteral("all"), false);
                return true;
            }
            if (index.isValid() && delegate_->eyeColumnContains(tree_->visualRect(index), pos)) {
                const QString path = pathForProxyIndex(index);
                const QString kind = index.data(KindRole).toString();
                if (view_ && kind.startsWith(QLatin1String("smart-filter"))) {
                    const int sf = path.indexOf(QLatin1String("/@sf"));
                    const QString base = sf >= 0 ? path.left(sf) : path;
                    const bool on = !index.data(VisibleRole).toBool();
                    if (kind == QLatin1String("smart-filters")) {
                        set_layer_smart_filters_enabled(*view_, base, on);
                    } else {
                        const int filterIndex =
                            path.mid(path.lastIndexOf(QLatin1Char('/')) + 1).toInt();
                        set_layer_smart_filter_visible(*view_, base, filterIndex, on);
                    }
                    return true;
                }
                if (mouse->modifiers() & Qt::AltModifier) {
                    toggleSolo(path);
                    return true;
                }
                QStringList targets = selectedPaths();
                if (!targets.contains(path)) {
                    targets = QStringList{path};
                }
                const bool on = !model_->visibleInDocument(path);
                if (view_ && !targets.isEmpty()) {
                    view_->set_layers_visible(targets, on);
                    // Patch the toggled rows in place so a second quick toggle
                    // reads the new value instead of a stale one.
                    for (const QString& target : targets) {
                        model_->setVisible(target, on);
                    }
                }
                return true;
            }
        }
    }
    if (watched == tree_->viewport() && event->type() == QEvent::MouseButtonDblClick) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            const QPoint pos = mouse->position().toPoint();
            const QModelIndex index = tree_->indexAt(pos);
            if (index.isValid()) {
                if (index.data(KindRole).toString().startsWith(QLatin1String("smart-filter"))) {
                    return true;
                }
                // The mask thumbnail and link glyph own their clicks; a
                // double-click there must not rename or open a style.
                const QRect rowRect = tree_->visualRect(index);
                if (delegate_->maskThumbRect(rowRect, index).contains(pos)
                    || delegate_->linkGlyphRect(rowRect, index).contains(pos)
                    || delegate_->vectorMaskThumbRect(rowRect, index).contains(pos)
                    || delegate_->vectorLinkGlyphRect(rowRect, index).contains(pos)) {
                    return true;
                }
                const QString path = pathForProxyIndex(index);
                if (index.data(KindRole).toString() == QLatin1String("background")) {
                    // A control keeps its own action; any other content-band
                    // double-click converts the Background through the dialog.
                    const QRect vr = tree_->visualRect(index);
                    const bool control = delegate_->eyeColumnContains(vr, pos)
                        || delegate_->thumbRect(vr, index).contains(pos);
                    if (!control) {
                        openBackgroundConversion(path);
                    }
                } else if (delegate_->nameRect(tree_->visualRect(index), index).contains(pos)) {
                    tree_->edit(index);
                } else if (!delegate_->eyeColumnContains(tree_->visualRect(index), pos)
                           && !delegate_->lockRect(tree_->visualRect(index)).contains(pos)
                           && !delegate_->fxRect(tree_->visualRect(index), index).contains(pos)) {
                    openLayerStyle(path);
                }
                return true;
            }
        }
    }
    return QWidget::eventFilter(watched, event);
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
    }
    const bool active = view_ && !paths.isEmpty();
    syncing_ = true;
    blend_->setEnabled(false);
    opacity_->setEnabled(false);
    fill_->setEnabled(false);
    const std::array<QToolButton*, 5> lockButtons = {
        lockTransparency_, lockPixels_, lockPosition_, lockNesting_, lockAll_};
    for (QToolButton* button : lockButtons) {
        button->setEnabled(false);
        button->setChecked(false);
    }
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
        bool lockAny = false;
        bool blendAny = false;
        bool opacityAny = false;
        bool fillAny = false;
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
        for (QToolButton* button : lockButtons) {
            button->setEnabled(lockAny);
        }
        // A type or shape layer forces Transparency and Image on: show them
        // checked (the row projection already sets the bits) and refuse a click.
        const bool forcedLocks = valueIndex.data(LayerRowShapeRole).toBool()
            || valueIndex.data(KindRole).toString() == QLatin1String("type");
        if (forcedLocks) {
            lockTransparency_->setEnabled(false);
            lockPixels_->setEnabled(false);
        }
        blend_->setEnabled(blendAny);
        opacity_->setEnabled(opacityAny);
        fill_->setEnabled(fillAny);
    }
    syncing_ = false;
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
    proxy_->setFilter(filter);
    if (filter.enabled && proxy_->hasActiveCriteria()) {
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

} // namespace pictura
