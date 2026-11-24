#pragma once

#include "frame.h"

#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QImage>

namespace pictura::selftest_fixture {

class DocumentCleanup {
public:
    PicturaMainWindow& frame;

    explicit DocumentCleanup(PicturaMainWindow& frame)
        : frame(frame), firstDocument_(frame.documentCount()) {}
    ~DocumentCleanup() { closeAll(); }
    DocumentCleanup(const DocumentCleanup&) = delete;
    DocumentCleanup& operator=(const DocumentCleanup&) = delete;

    void closeAll()
    {
        if (closed_) {
            return;
        }
        closed_ = true;
        while (frame.documentCount() > firstDocument_) {
            if (!frame.closeDocument(frame.documentCount() - 1, false)) {
                break;
            }
        }
    }

private:
    int firstDocument_;
    bool closed_ = false;
};

class DocumentFixture : public DocumentCleanup {
public:
    QString path;
    PictureView* view = nullptr;
    ImageView* canvas = nullptr;
    int doc = -1;

    DocumentFixture(PicturaMainWindow& frame, const QImage& seed, const QString& name)
        : DocumentCleanup(frame)
        , path(QDir::temp().filePath(name + QStringLiteral(".png")))
    {
        if (!frame.newDocument(name, seed.width(), seed.height(), QStringLiteral("rgb"), 8,
                               QStringLiteral("white"))) {
            return;
        }
        doc = frame.activeDocumentIndex();
        PictureView* opened = frame.activeView();
        if (opened && seed.save(path, "PNG") && opened->open_image(path)) {
            view = opened;
            canvas = frame.imageView();
        }
    }

    ~DocumentFixture() { QFile::remove(path); }

    bool ok() const { return view && canvas; }
};

}  // namespace pictura::selftest_fixture
