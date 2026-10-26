#pragma once

#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtGui/QIconEngine>

namespace pictura {

// Renders a bundled SVG icon at the requested size and device pixel ratio,
// recolouring by the rendered alpha with the active theme's icon tint (or an
// explicit colour). Folding the theme generation into key() lets QIcon
// re-render after a brightness change.
class SvgIconEngine : public QIconEngine
{
public:
    explicit SvgIconEngine(QString resourcePath, const QColor& color = QColor());

    void paint(QPainter* painter, const QRect& rect, QIcon::Mode mode,
               QIcon::State state) override;
    QPixmap pixmap(const QSize& size, QIcon::Mode mode, QIcon::State state) override;
    QPixmap scaledPixmap(const QSize& size, QIcon::Mode mode, QIcon::State state,
                         qreal scale) override;
    QString key() const override;
    bool isNull() override;
    QIconEngine* clone() const override;

private:
    QPixmap render(const QSize& pixelSize, QIcon::Mode mode) const;
    QColor colorFor(QIcon::Mode mode) const;

    QString resourcePath_;
    QColor color_;
};

} // namespace pictura
