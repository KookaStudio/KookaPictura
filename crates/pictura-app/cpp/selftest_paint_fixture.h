#pragma once

// The document fixture and brush-state guard the paint-tool self-tests share.

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QImage>

namespace paint_fixture {

// A document opened from `seed`, closed (and its PNG removed) on scope exit.
struct Fixture {
    pictura::PicturaMainWindow& frame;
    QString path;
    pictura::PictureView* view = nullptr;
    pictura::ImageView* canvas = nullptr;
    pictura::ToolController* tools = nullptr;
    int doc = -1;

    Fixture(pictura::PicturaMainWindow& f, const QImage& seed, const QString& name)
        : frame(f)
        , path(QDir::temp().filePath(name + QStringLiteral(".png")))
    {
        if (!frame.newDocument(name, seed.width(), seed.height(), QStringLiteral("rgb"), 8,
                               QStringLiteral("white"))) {
            return;
        }
        doc = frame.activeDocumentIndex();
        pictura::PictureView* v = frame.activeView();
        if (v && seed.save(path, "PNG") && v->open_image(path)) {
            view = v;
            canvas = frame.imageView();
            tools = frame.findChild<pictura::ToolController*>();
        }
    }
    ~Fixture()
    {
        frame.setActiveTool(pictura::ToolId::Move);
        if (doc >= 0) {
            frame.closeDocument(doc, false);
        }
        QFile::remove(path);
    }
    bool ok() const { return view && canvas && tools; }

    void drag(const QList<QPointF>& points, Qt::KeyboardModifiers mods = Qt::NoModifier) const
    {
        canvas->mousePressed(points.first(), Qt::LeftButton, int(mods));
        for (const QPointF& p : points.mid(1)) {
            canvas->mouseMoved(p);
        }
        canvas->mouseReleased(points.last());
    }

    bool committedOnce(int base, const char* label) const
    {
        return view->history_index() == base + 1
            && view->history_label(base + 1) == QString::fromLatin1(label);
    }
};


// Brush settings a stamp check changes, restored on scope exit.
struct BrushState {
    pictura::ToolController* tools;
    int size;
    int opacity;
    int flow;
    pictura::StampOptions stamp;

    explicit BrushState(pictura::ToolController* t)
        : tools(t)
        , size(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->size())
        , opacity(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->opacity())
        , flow(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->flow())
        , stamp(t->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp)->get())
    {
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(6);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setOpacity(100);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setFlow(100);
    }
    ~BrushState()
    {
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(size);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setOpacity(opacity);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setFlow(flow);
        tools->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp)->set(stamp);
    }
};

} // namespace paint_fixture
