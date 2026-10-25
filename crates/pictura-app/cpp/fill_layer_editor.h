#pragma once

#include <QtCore/QString>
#include <QtGui/QColor>

class QWidget;

namespace pictura {

class PictureView;

// Open the fill type's editor on `path` and apply the accepted content in one
// undo state. False when the path is not an editable fill or the user cancels.
bool openFillLayerEditor(PictureView& view, const QString& path, QWidget* parent);

// Create a fill layer of `kind` through the editor flow. `showNewLayerDialog`
// is true for the Layer menu (New Layer dialog, then the type editor, then one
// creation); false for the panel quick-add (create with defaults, then open the
// editor on the new layer). `solid` is the default Solid Color content.
bool createFillLayerFromDialog(PictureView& view, const QString& kind, const QColor& solid,
                               QWidget* parent, bool showNewLayerDialog);

} // namespace pictura
