#include "svg_icon_engine.h"

#include <QtCore/QFile>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPixmap>
#include <QtSvg/QSvgRenderer>
#include <QtWidgets/QApplication>

#include <utility>

#include "theme.h"

namespace pictura {

SvgIconEngine::SvgIconEngine(QString resourcePath, const QColor& color)
    : resourcePath_(std::move(resourcePath))
    , color_(color)
{
}

QColor SvgIconEngine::colorFor(QIcon::Mode mode) const
{
    if (color_.isValid()) {
        return color_;
    }
    if (mode == QIcon::Disabled) {
        const QPalette pal = QApplication::palette();
        return pal.color(QPalette::Disabled, QPalette::ButtonText);
    }
    return Theme::tokenColor(QStringLiteral("icon"));
}

QPixmap SvgIconEngine::render(const QSize& pixelSize, QIcon::Mode mode) const
{
    QPixmap pixmap(pixelSize);
    pixmap.fill(Qt::transparent);

    QSvgRenderer renderer(resourcePath_);
    QPainter painter(&pixmap);
    // Fit the SVG inside the canvas preserving its aspect ratio (letterboxed and
    // centred), so a square icon is never squashed into a non-square canvas.
    const QSizeF canvas(pixelSize);
    QSizeF fitted = QSizeF(renderer.defaultSize()).scaled(canvas, Qt::KeepAspectRatio);
    if (fitted.isEmpty()) {
        fitted = canvas;
    }
    const QRectF target((canvas.width() - fitted.width()) / 2.0,
                        (canvas.height() - fitted.height()) / 2.0, fitted.width(),
                        fitted.height());
    renderer.render(&painter, target);
    // SourceIn keeps the rendered alpha and replaces the colour, so alpha
    // gradients in the source become gradients of the tint. The whole set sits
    // at 80% opacity so it reads a touch softer against the chrome.
    painter.setCompositionMode(QPainter::CompositionMode_SourceIn);
    painter.setOpacity(0.8);
    painter.fillRect(pixmap.rect(), colorFor(mode));
    painter.end();
    return pixmap;
}

void SvgIconEngine::paint(QPainter* painter, const QRect& rect, QIcon::Mode mode,
                          QIcon::State /*state*/)
{
    const qreal scale = painter->device() ? painter->device()->devicePixelRatioF() : 1.0;
    const QSize pixel(qMax(1, qRound(rect.width() * scale)),
                      qMax(1, qRound(rect.height() * scale)));
    QPixmap pixmap = render(pixel, mode);
    pixmap.setDevicePixelRatio(scale);
    painter->drawPixmap(rect, pixmap);
}

QPixmap SvgIconEngine::scaledPixmap(const QSize& size, QIcon::Mode mode,
                                    QIcon::State /*state*/, qreal scale)
{
    const QSize pixel(qMax(1, qRound(size.width() * scale)),
                      qMax(1, qRound(size.height() * scale)));
    QPixmap pixmap = render(pixel, mode);
    pixmap.setDevicePixelRatio(scale);
    return pixmap;
}

QPixmap SvgIconEngine::pixmap(const QSize& size, QIcon::Mode mode, QIcon::State state)
{
    return scaledPixmap(size, mode, state, 1.0);
}

QString SvgIconEngine::key() const
{
    return resourcePath_ + QLatin1Char('|') + color_.name(QColor::HexArgb)
           + QLatin1Char('|') + QString::number(Theme::paletteGeneration());
}

bool SvgIconEngine::isNull()
{
    return !QFile::exists(resourcePath_);
}

QIconEngine* SvgIconEngine::clone() const
{
    return new SvgIconEngine(resourcePath_, color_);
}

} // namespace pictura
