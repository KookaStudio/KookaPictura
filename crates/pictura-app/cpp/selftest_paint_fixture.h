#pragma once

#include "selftest_fixture.h"
#include "image_view.h"
#include "tools.h"

#include "api/view_bridge.h"

namespace paint_fixture {

struct Fixture : pictura::selftest_fixture::DocumentFixture {
    pictura::ToolController* tools = nullptr;

    Fixture(pictura::PicturaMainWindow& frame, const QImage& seed, const QString& name)
        : pictura::selftest_fixture::DocumentFixture(frame, seed, name)
    {
        if (view) {
            tools = frame.findChild<pictura::ToolController*>();
        }
    }

    ~Fixture() { frame.setActiveTool(pictura::ToolId::Move); }

    bool ok() const { return DocumentFixture::ok() && tools; }

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
        return view->history_index() == base + 1 && view->history_label(base + 1) == QString::fromLatin1(label);
    }
};

struct BrushState {
    pictura::ToolController* tools;
    int size;
    int opacity;
    int flow;
    bool airbrush;
    bool pressureSize;
    bool pressureOpacity;
    double pressure;
    pictura::StampOptions stamp;

    explicit BrushState(pictura::ToolController* t)
        : tools(t)
        , size(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->size())
        , opacity(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->opacity())
        , flow(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->flow())
        , airbrush(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->airbrush())
        , pressureSize(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->pressureSize())
        , pressureOpacity(t->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->pressureOpacity())
        , pressure(t->pressure())
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
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setAirbrush(airbrush);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setPressureSize(pressureSize);
        tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setPressureOpacity(pressureOpacity);
        tools->setPressure(pressure);
        tools->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp)->set(stamp);
    }
};

}  // namespace paint_fixture
