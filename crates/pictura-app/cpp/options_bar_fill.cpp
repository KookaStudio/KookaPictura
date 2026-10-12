// The options bars of the fill tools: Gradient and Paint Bucket. Part of
// OptionsBar; split from options_bar.cpp along the page seam.

#include "options_bar.h"

#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include <QtGui/QActionGroup>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <algorithm>
#include <cmath>
#include <cstdint>

namespace pictura {

namespace {

const QSize kSampleSize(64, 16);

const char* const kGradientStyles[] = {"Linear Gradient", "Radial Gradient", "Angle Gradient",
                                       "Reflected Gradient", "Diamond Gradient"};

// Built-in gradient `index` between the two colours, over a checkerboard so
// its transparent stretches read as transparent.
QIcon gradientIcon(int index, const QColor& foreground, const QColor& background)
{
    const int w = kSampleSize.width();
    const int h = kSampleSize.height();
    const ::rust::Vec<std::uint8_t> rgba =
        gradient_preset_strip(index, foreground.rgba(), background.rgba(), w, h);
    if (rgba.size() != static_cast<size_t>(w) * h * 4) {
        return QIcon();
    }
    QPixmap pixmap(kSampleSize);
    pixmap.fill(Qt::white);
    QPainter painter(&pixmap);
    const int cell = 4;
    for (int y = 0; y < h; y += cell) {
        for (int x = (y / cell % 2) * cell; x < w; x += 2 * cell) {
            painter.fillRect(x, y, cell, cell, QColor(0xcc, 0xcc, 0xcc));
        }
    }
    painter.drawImage(0, 0, QImage(rgba.data(), w, h, w * 4, QImage::Format_RGBA8888));
    return QIcon(pixmap);
}

// A 16 px black-to-white swatch of gradient style `style`, dragged from the
// centre (from the left edge for Linear) to the right edge.
QIcon gradientStyleIcon(int style)
{
    const int side = 16;
    const double centre = (side - 1) / 2.0;
    QImage image(side, side, QImage::Format_RGB32);
    for (int y = 0; y < side; ++y) {
        for (int x = 0; x < side; ++x) {
            const double dx = x - centre;
            const double dy = y - centre;
            double t = 0.0;
            switch (style) {
            case 0:
                t = x / (side - 1.0);
                break;
            case 1:
                t = std::hypot(dx, dy) / centre;
                break;
            case 2:
                t = std::atan2(-dy, dx) / (2.0 * M_PI);
                t = t < 0.0 ? t + 1.0 : t;
                break;
            case 3:
                t = std::abs(dx) / centre;
                break;
            default:
                t = (std::abs(dx) + std::abs(dy)) / centre;
                break;
            }
            const int v = qRound(26 + std::clamp(t, 0.0, 1.0) * 214);
            image.setPixel(x, y, qRgb(v, v, v));
        }
    }
    return QIcon(QPixmap::fromImage(image));
}

// The Brush modes a fill offers; Clear only where CS6 has it (the Paint Bucket).
QComboBox* addFillMode(QHBoxLayout* layout, QWidget* page, const QString& name,
                       const QString& current, bool withClear)
{
    layout->addWidget(new QLabel(QStringLiteral("Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(name);
    mode->addItem(QStringLiteral("Normal"), QStringLiteral("normal"));
    mode->addItem(QStringLiteral("Dissolve"), QStringLiteral("dissolve"));
    mode->addItem(QStringLiteral("Behind"), QStringLiteral("behind"));
    if (withClear) {
        mode->addItem(QStringLiteral("Clear"), QStringLiteral("clear"));
    }
    mode->setCurrentIndex(std::max(0, mode->findData(current)));
    layout->addWidget(mode);
    return mode;
}

QCheckBox* addFillToggle(QHBoxLayout* layout, QWidget* page, const QString& label,
                         const QString& name, bool checked)
{
    auto* box = new QCheckBox(label, page);
    box->setObjectName(name);
    box->setChecked(checked);
    layout->addWidget(box);
    return box;
}

} // namespace

// CS6's Gradient bar: the gradient sample with its preset menu, the five style
// buttons, Mode, Opacity, Reverse, Dither, and Transparency.
// ponytail: no Gradient Editor; the sample picks among the built-in gradients.
QWidget* OptionsBar::buildGradientPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    const GradientOptions initial =
        controller_ ? gradient_->get() : GradientOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            GradientOptions o = gradient_->get();
            edit(o);
            gradient_->set(o);
        }
    };

    auto* sample = new QToolButton(page);
    sample->setObjectName(QStringLiteral("optionsGradientSample"));
    sample->setPopupMode(QToolButton::InstantPopup);
    sample->setIconSize(kSampleSize);
    auto* presets = new QMenu(sample);
    auto* group = new QActionGroup(presets);
    for (int i = 0; i < gradient_preset_count(); ++i) {
        QAction* action = presets->addAction(gradient_preset_name(i));
        action->setCheckable(true);
        action->setChecked(i == initial.preset);
        group->addAction(action);
    }
    sample->setMenu(presets);
    layout->addWidget(sample);
    // The first presets follow the foreground and background colours, so the
    // sample and the menu are redrawn when either changes.
    const auto refresh = [this, sample, presets] {
        const QColor fg = controller_ ? controller_->foreground() : QColor(Qt::black);
        const QColor bg = controller_ ? controller_->background() : QColor(Qt::white);
        const QList<QAction*> actions = presets->actions();
        for (int i = 0; i < actions.size(); ++i) {
            actions.at(i)->setIcon(gradientIcon(i, fg, bg));
            if (actions.at(i)->isChecked()) {
                sample->setIcon(actions.at(i)->icon());
                sample->setToolTip(actions.at(i)->text());
            }
        }
    };
    refresh();
    connect(group, &QActionGroup::triggered, this, [update, refresh, presets](QAction* action) {
        const int index = presets->actions().indexOf(action);
        update([index](GradientOptions& o) { o.preset = index; });
        refresh();
    });
    if (controller_) {
        connect(controller_, &ToolController::colorsChanged, sample, refresh);
    }

    auto* styles = new QButtonGroup(page);
    for (int i = 0; i < 5; ++i) {
        auto* button = new QToolButton(page);
        button->setObjectName(QStringLiteral("optionsGradientStyle%1").arg(i));
        button->setIcon(gradientStyleIcon(i));
        button->setToolTip(QString::fromLatin1(kGradientStyles[i]));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setChecked(i == initial.style);
        styles->addButton(button, i);
        layout->addWidget(button);
    }
    connect(styles, &QButtonGroup::idClicked, this,
            [update](int i) { update([i](GradientOptions& o) { o.style = i; }); });

    auto* mode = addFillMode(layout, page, QStringLiteral("optionsGradientMode"), initial.mode,
                             false);
    connect(mode, &QComboBox::currentIndexChanged, this, [update, mode](int) {
        const QString name = mode->currentData().toString();
        update([name](GradientOptions& o) { o.mode = name; });
    });

    auto* opacity = new NumericField(
        QStringLiteral("Opacity:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsGradientOpacity")),
        page);
    opacity->setValue(initial.opacity);
    layout->addWidget(opacity);
    connect(opacity, &NumericField::valueChanged, this, [update](double v) {
        update([v](GradientOptions& o) { o.opacity = qRound(v); });
    });

    auto* reverse = addFillToggle(layout, page, QStringLiteral("Reverse"),
                                  QStringLiteral("optionsGradientReverse"), initial.reverse);
    connect(reverse, &QCheckBox::toggled, this,
            [update](bool on) { update([on](GradientOptions& o) { o.reverse = on; }); });
    auto* dither = addFillToggle(layout, page, QStringLiteral("Dither"),
                                 QStringLiteral("optionsGradientDither"), initial.dither);
    connect(dither, &QCheckBox::toggled, this,
            [update](bool on) { update([on](GradientOptions& o) { o.dither = on; }); });
    auto* transparency =
        addFillToggle(layout, page, QStringLiteral("Transparency"),
                      QStringLiteral("optionsGradientTransparency"), initial.transparency);
    connect(transparency, &QCheckBox::toggled, this,
            [update](bool on) { update([on](GradientOptions& o) { o.transparency = on; }); });

    layout->addStretch(1);
    return page;
}

// CS6's Paint Bucket bar: Fill (Foreground / Pattern) with the pattern picker,
// Mode, Opacity, Tolerance (32, the Magic Wand's scale), Anti-alias,
// Contiguous, and All Layers.
QWidget* OptionsBar::buildPaintBucketPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    const BucketOptions initial = controller_ ? bucket_->get() : BucketOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            BucketOptions o = bucket_->get();
            edit(o);
            bucket_->set(o);
        }
    };

    auto* fill = new QComboBox(page);
    fill->setObjectName(QStringLiteral("optionsBucketFill"));
    fill->setToolTip(QStringLiteral("Set source for fill area"));
    fill->addItems({QStringLiteral("Foreground"), QStringLiteral("Pattern")});
    fill->setCurrentIndex(initial.fill);
    layout->addWidget(fill);
    auto* pattern = new QComboBox(page);
    pattern->setObjectName(QStringLiteral("optionsBucketPattern"));
    pattern->setToolTip(QStringLiteral("Pattern"));
    pattern->setIconSize(QSize(20, 20));
    for (int i = 0; i < stamp_pattern_count(); ++i) {
        pattern->addItem(patternIcon(i), stamp_pattern_name(i));
    }
    pattern->setCurrentIndex(initial.pattern);
    pattern->setEnabled(initial.fill == 1);
    layout->addWidget(pattern);
    connect(fill, &QComboBox::currentIndexChanged, this, [update, pattern](int i) {
        pattern->setEnabled(i == 1);
        update([i](BucketOptions& o) { o.fill = i; });
    });
    connect(pattern, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](BucketOptions& o) { o.pattern = i; }); });

    auto* mode =
        addFillMode(layout, page, QStringLiteral("optionsBucketMode"), initial.mode, true);
    connect(mode, &QComboBox::currentIndexChanged, this, [update, mode](int) {
        const QString name = mode->currentData().toString();
        update([name](BucketOptions& o) { o.mode = name; });
    });

    auto* opacity = new NumericField(
        QStringLiteral("Opacity:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsBucketOpacity")),
        page);
    opacity->setValue(initial.opacity);
    layout->addWidget(opacity);
    connect(opacity, &NumericField::valueChanged, this, [update](double v) {
        update([v](BucketOptions& o) { o.opacity = qRound(v); });
    });
    auto* tolerance = new NumericField(
        QStringLiteral("Tolerance:"),
        numericConfig(0, 255, 1, 0, QString(), true, QStringLiteral("optionsBucketTolerance")),
        page);
    tolerance->setValue(initial.tolerance);
    layout->addWidget(tolerance);
    connect(tolerance, &NumericField::valueChanged, this, [update](double v) {
        update([v](BucketOptions& o) { o.tolerance = qRound(v); });
    });

    auto* antialias = addFillToggle(layout, page, QStringLiteral("Anti-alias"),
                                    QStringLiteral("optionsBucketAntialias"), initial.antialias);
    connect(antialias, &QCheckBox::toggled, this,
            [update](bool on) { update([on](BucketOptions& o) { o.antialias = on; }); });
    auto* contiguous =
        addFillToggle(layout, page, QStringLiteral("Contiguous"),
                      QStringLiteral("optionsBucketContiguous"), initial.contiguous);
    connect(contiguous, &QCheckBox::toggled, this,
            [update](bool on) { update([on](BucketOptions& o) { o.contiguous = on; }); });
    auto* allLayers = addFillToggle(layout, page, QStringLiteral("All Layers"),
                                    QStringLiteral("optionsBucketAllLayers"), initial.allLayers);
    connect(allLayers, &QCheckBox::toggled, this,
            [update](bool on) { update([on](BucketOptions& o) { o.allLayers = on; }); });

    layout->addStretch(1);
    return page;
}

} // namespace pictura
