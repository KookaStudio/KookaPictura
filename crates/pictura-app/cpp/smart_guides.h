#pragma once

// Smart alignment guides: while the Move tool drags a layer, or the Type tool
// drags text, snap the moving content box to the document's edges/centre or to
// another layer's content edges/centres, and report the magenta guides and gap
// distances to draw. The geometry is pure so it is tested without a canvas.

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/QSizeF>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

class PictureView;

struct SmartGuideLine {
    bool vertical = false;
    double position = 0.0;

    bool operator==(const SmartGuideLine& other) const
    {
        return vertical == other.vertical && position == other.position;
    }
};

// The distance between the moving and reference content boxes along a snapped
// guide's axis, with the document-space point to draw the label at.
struct SmartGuideGap {
    bool verticalGuide = false;
    double distance = 0.0;
    QPointF anchor;
};

struct SmartGuideResult {
    // Document-pixel adjustment to add to the moving box's position.
    QPointF snap;
    QList<SmartGuideLine> lines;
    QList<SmartGuideGap> gaps;

    bool active() const { return !lines.isEmpty(); }
};

// `moving` is the moving content box in document pixels at the pointer's raw
// offset; `references` are other layers' content boxes; `document` is the
// canvas size; `zoom` is screen pixels per document pixel. An edge snaps when
// it is within `thresholdScreenPx` screen pixels of a target.
SmartGuideResult smartGuideSnap(const QRectF& moving, const QList<QRectF>& references,
                                const QSizeF& document, double zoom,
                                double thresholdScreenPx = 8.0);

// Content boxes of the layers that can be references while `movingPaths` move:
// every visible layer other than those paths and their descendants that has ink.
QList<QRectF> smartGuideReferences(PictureView& view, const QStringList& movingPaths);

} // namespace pictura
