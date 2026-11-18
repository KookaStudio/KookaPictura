#pragma once

#include <QtCore/QStringList>

class QByteArray;

namespace pictura {

class PicturaMainWindow;
class PictureView;

// File > Place: place each path in order, settling one Free Transform session
// per placed layer. A session ended by Escape or the transform's Cancel control
// removes that placement; any other non-interruption end keeps it. See the
// multi-file-place change.
void placeDocuments(PicturaMainWindow& frame, const QStringList& paths);

QString importPdfArtwork(PictureView& view, const QByteArray& bytes,
                         const QString& filename, bool smart, unsigned int page,
                         unsigned char crop, const QString& label, QString& error);

} // namespace pictura
