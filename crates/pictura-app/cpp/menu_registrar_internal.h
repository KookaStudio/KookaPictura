#pragma once

// Small helpers shared by the per-menu implementation files. They read the
// collaborators directly, so the menu translation units never need the main-window header.

#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>

#include "api/layers_bridge.h"
#include "commands.h"
#include "dialogs.h"
#include "document_registry.h"
#include "image_view.h"
#include "tools.h"

namespace pictura::menu_internal {

inline PictureView* activeView(DocumentRegistry* documents)
{
    return documents ? documents->activeView() : nullptr;
}

inline ImageView* imageView(DocumentRegistry* documents)
{
    return documents ? documents->imageView() : nullptr;
}

inline QStringList selectedLayerPaths(PictureView* view)
{
    return view ? selected_layer_paths(*view) : QStringList{};
}

inline QString layerAnchor(PictureView* view)
{
    return view ? layer_selection_anchor(*view) : QString{};
}

} // namespace pictura::menu_internal
