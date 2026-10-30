#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>

namespace pictura {

class PictureView;
class PicturaMainWindow;

namespace control_detail {

QJsonObject documentInfo(PicturaMainWindow* frame, int index);

// Stable for a document's life, unlike its index: clients pass it back as `doc`.
QString documentId(const PictureView* view);

// Why `path` may not be opened (or, when `writes`, saved to), or empty. The
// session token already limits callers to the same user, so any absolute
// path is allowed; these refuse only spellings that land somewhere other
// than the one named (#646).
QString pathProblem(const QString& path, bool writes);

} // namespace control_detail

} // namespace pictura
