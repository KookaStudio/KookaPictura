#pragma once

#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QString>
#include <QtGui/QColor>

namespace pictura {

struct CharacterSetting;
struct ParagraphSetting;

// Session-command ports a tool handler may implement alongside ToolHandler.
// `ToolController` resolves one typed (`cropCommitSession`, `cropBoxSession`,
// `perspectiveCropSession`, `textSession`) with `dynamic_cast`, so RTTI must
// stay enabled for the app target (no `-fno-rtti`, no hidden visibility on the
// handler classes). Resolve-and-discard: handlers outlive tool switches today,
// but a future registry rebuild would stale a cached pointer.

// Crop and Perspective Crop both commit the region they have staged; the
// resolver picks the active one (Perspective Crop's quad lives only while its
// tool is active, Crop's staged box survives a switch).
class CropCommitSession {
public:
    virtual ~CropCommitSession() = default;
    virtual bool commitCrop() = 0;
};

// The Crop tool's box surface; Perspective Crop does not implement it.
class CropBoxSession {
public:
    virtual ~CropBoxSession() = default;
    virtual bool cropActive() const = 0;
    virtual bool resizeCrop(double width, double height) = 0;
    virtual double cropWidth() const = 0;
    virtual double cropHeight() const = 0;
    virtual bool resetCropBox() = 0;
    virtual QPointF cropCenter() const = 0;
    virtual void setStraightenMode(bool on) = 0;
    virtual bool hasPendingCrop() const = 0;
    virtual QRect pendingCropRect() const = 0;
};

// Perspective Crop's own reads.
class PerspectiveCropSession {
public:
    virtual ~PerspectiveCropSession() = default;
    virtual void setPerspectiveOutput(int width, int height) = 0;
    virtual bool perspectiveQuadComplete() const = 0;
    virtual void setPerspectiveShowGrid(bool on) = 0;
    virtual void resetPerspectiveQuad() = 0;
};

// The active type tool's edit session.
class TextSession {
public:
    virtual ~TextSession() = default;
    virtual bool commitText() = 0;
    virtual bool cancelText() = 0;
    virtual bool textActive() const = 0;
    virtual bool insertText(const QString& text) = 0;
    // The open session's attributes, which the Character and Paragraph panels
    // edit in place of any layer; false with no session open. Setting them
    // previews at once, records no state, and lands on commit.
    virtual bool textCharacter(CharacterSetting& character, QString& family) const = 0;
    virtual bool setTextCharacter(const CharacterSetting& character, const QString& family) = 0;
    virtual bool textParagraph(ParagraphSetting& paragraph) const = 0;
    virtual bool setTextParagraph(const ParagraphSetting& paragraph) = 0;
    // Shows `color` on the text being typed or the selected type layer without
    // a history state (the Text Color picker's live preview); calling it with
    // the original colour drops the preview. The committed colour still goes
    // through the options bar.
    virtual bool previewTextColor(const QColor& color) = 0;
};

} // namespace pictura
