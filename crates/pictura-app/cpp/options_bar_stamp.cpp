// The options bars of the source-painting tools: Clone Stamp, Pattern Stamp,
// History Brush, Art History Brush, and the Eraser. Part of OptionsBar; split
// from options_bar.cpp along the page seam.

#include "options_bar.h"

#include "commands.h"
#include "icons.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <algorithm>
#include <cstdint>
#include <functional>

namespace pictura {

QIcon patternIcon(int index)
{
    const int side = stamp_pattern_side();
    const ::rust::Vec<std::uint8_t> rgba = stamp_pattern_tile(index);
    if (rgba.size() != static_cast<size_t>(side) * side * 4) {
        return QIcon();
    }
    const QImage tile(rgba.data(), side, side, side * 4, QImage::Format_RGBA8888);
    return QIcon(QPixmap::fromImage(tile.copy()));
}

// The Airbrush option: paint keeps building while the button is held still.
static void addAirbrushToggle(QHBoxLayout* layout, QWidget* page, const QString& name, bool on,
                              std::function<void(bool)> setter)
{
    auto* button = new QToolButton(page);
    button->setObjectName(name);
    button->setText(QStringLiteral("Airbrush"));
    button->setToolTip(QStringLiteral("Build up paint while the mouse button is held"));
    button->setCheckable(true);
    button->setChecked(on);
    button->setAutoRaise(true);
    layout->addWidget(button);
    QObject::connect(button, &QToolButton::toggled, page, std::move(setter));
}

// A 0-100 % field wired to a controller setter; returns it.
NumericField* OptionsBar::addPercentField(QHBoxLayout* layout, QWidget* page,
                                          const QString& label, const QString& name, int value,
                                          std::function<void(int)> setter)
{
    auto* field = new NumericField(
        label, numericConfig(0, 100, 1, 0, QStringLiteral("%"), true, name), page);
    field->setValue(value);
    layout->addWidget(field);
    if (controller_) {
        connect(field, &NumericField::valueChanged, this,
                [setter](double v) { setter(qRound(v)); });
    }
    return field;
}

// The Brush's tip, Mode, Opacity, and Flow, shared through the same controller
// fields. The stamps' Mode lists the Brush modes that make sense for copied
// pixels (no Clear).
void OptionsBar::addStampPaintFields(QHBoxLayout* layout, QWidget* page)
{
    addBrushTipFields(layout, page);

    layout->addWidget(new QLabel(QStringLiteral("Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsStampMode"));
    mode->addItem(QStringLiteral("Normal"), QStringLiteral("normal"));
    mode->addItem(QStringLiteral("Dissolve"), QStringLiteral("dissolve"));
    mode->addItem(QStringLiteral("Behind"), QStringLiteral("behind"));
    layout->addWidget(mode);

    addPercentField(layout, page, QStringLiteral("Opacity:"), QStringLiteral("optionsStampOpacity"),
                    controller_ ? paint_->opacity() : 100,
                    [this](int v) { paint_->setOpacity(v); });
    addPercentField(layout, page, QStringLiteral("Flow:"), QStringLiteral("optionsStampFlow"),
                    controller_ ? paint_->flow() : 100, [this](int v) { paint_->setFlow(v); });

    if (controller_) {
        mode->setCurrentIndex(std::max(0, mode->findData(paint_->mode())));
        connect(mode, &QComboBox::currentIndexChanged, this, [this, mode](int) {
            paint_->setMode(mode->currentData().toString());
        });
    }
}

// CS6's Clone Stamp and Pattern Stamp bars: the paint fields plus Aligned and
// Sample (with Ignore Adjustment Layers beside All Layers), or the pattern
// picker, Aligned, and Impressionist.
QWidget* OptionsBar::buildStampPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addStampPaintFields(layout, page);

    const StampOptions initial = controller_ ? stamp_->get() : StampOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            StampOptions o = stamp_->get();
            edit(o);
            stamp_->set(o);
        }
    };
    const bool clone = id == ToolId::CloneStamp;
    addAirbrushToggle(layout, page,
                      clone ? QStringLiteral("optionsCloneAirbrush")
                            : QStringLiteral("optionsPatternAirbrush"),
                      clone ? initial.cloneAirbrush : initial.patternAirbrush,
                      [update, clone](bool on) {
                          update([on, clone](StampOptions& o) {
                              (clone ? o.cloneAirbrush : o.patternAirbrush) = on;
                          });
                      });
    if (clone) {
        // CS6 puts the Brush and Clone Source panel toggles right after the tip.
        auto addToggle = [&](const char* iconId, const QString& tip, const QString& name,
                             const QString& panel) {
            auto* button = new QToolButton(page);
            button->setObjectName(name);
            button->setIcon(pictura::icon(QString::fromLatin1(iconId)));
            button->setToolTip(tip);
            button->setAutoRaise(true);
            layout->insertWidget(3, button);
            connect(button, &QToolButton::clicked, this,
                    [this, panel] { emit panelToggleRequested(panel); });
        };
        addToggle(command_ids::WindowPanelsCloneSource, QStringLiteral("Toggle the Clone Source panel"),
                  QStringLiteral("optionsToggleCloneSourcePanel"),
                  QStringLiteral("cloneSourcePanel"));
        addToggle(command_ids::WindowPanelsBrush, QStringLiteral("Toggle the Brush panel"),
                  QStringLiteral("optionsToggleBrushPanel"), QStringLiteral("brushPanel"));
    }

    if (!clone) {
        auto* pattern = new QComboBox(page);
        pattern->setObjectName(QStringLiteral("optionsPatternStampPattern"));
        pattern->setToolTip(QStringLiteral("Pattern"));
        pattern->setIconSize(QSize(20, 20));
        for (int i = 0; i < stamp_pattern_count(); ++i) {
            pattern->addItem(patternIcon(i), stamp_pattern_name(i));
        }
        pattern->setCurrentIndex(initial.pattern);
        layout->addWidget(pattern);
        connect(pattern, &QComboBox::currentIndexChanged, this,
                [update](int i) { update([i](StampOptions& o) { o.pattern = i; }); });
    }

    auto* aligned = new QCheckBox(QStringLiteral("Aligned"), page);
    aligned->setObjectName(clone ? QStringLiteral("optionsCloneAligned")
                                 : QStringLiteral("optionsPatternAligned"));
    aligned->setChecked(clone ? initial.cloneAligned : initial.patternAligned);
    layout->addWidget(aligned);
    connect(aligned, &QCheckBox::toggled, this, [update, clone](bool on) {
        update([on, clone](StampOptions& o) { (clone ? o.cloneAligned : o.patternAligned) = on; });
    });

    if (clone) {
        layout->addWidget(new QLabel(QStringLiteral("Sample:"), page));
        auto* sample = new QComboBox(page);
        sample->setObjectName(QStringLiteral("optionsCloneSample"));
        sample->addItems({QStringLiteral("Current Layer"), QStringLiteral("Current & Below"),
                          QStringLiteral("All Layers")});
        sample->setCurrentIndex(initial.cloneSample);
        layout->addWidget(sample);
        auto* ignore = new QCheckBox(QStringLiteral("Ignore Adjustment Layers"), page);
        ignore->setObjectName(QStringLiteral("optionsCloneIgnoreAdjustments"));
        ignore->setChecked(initial.ignoreAdjustments);
        ignore->setVisible(initial.cloneSample == 2);
        layout->addWidget(ignore);
        connect(sample, &QComboBox::currentIndexChanged, this, [update, ignore](int i) {
            ignore->setVisible(i == 2);
            update([i](StampOptions& o) { o.cloneSample = i; });
        });
        connect(ignore, &QCheckBox::toggled, this,
                [update](bool on) { update([on](StampOptions& o) { o.ignoreAdjustments = on; }); });
    } else {
        auto* impressionist = new QCheckBox(QStringLiteral("Impressionist"), page);
        impressionist->setObjectName(QStringLiteral("optionsPatternImpressionist"));
        impressionist->setToolTip(QStringLiteral("Impressionist: not implemented yet"));
        impressionist->setEnabled(false);
        layout->addWidget(impressionist);
    }

    layout->addStretch(1);
    return page;
}

// CS6's History Brush bar: the paint fields. The source is chosen in the
// History panel.
QWidget* OptionsBar::buildHistoryBrushPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addStampPaintFields(layout, page);
    addAirbrushToggle(layout, page, QStringLiteral("optionsHistoryAirbrush"),
                      controller_ && stamp_->get().historyAirbrush, [this](bool on) {
                          if (controller_) {
                              StampOptions o = stamp_->get();
                              o.historyAirbrush = on;
                              stamp_->set(o);
                          }
                      });
    layout->addStretch(1);
    return page;
}

// CS6's Art History Brush bar: the tip, Mode, Opacity, Style, Area, and
// Tolerance. The source is the History Brush's, chosen in the History panel.
QWidget* OptionsBar::buildArtHistoryBrushPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    layout->addWidget(new QLabel(QStringLiteral("Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsArtHistoryMode"));
    mode->addItem(QStringLiteral("Normal"));
    mode->setToolTip(QStringLiteral("Blending modes: not implemented yet"));
    mode->setEnabled(false);
    layout->addWidget(mode);
    addPercentField(layout, page, QStringLiteral("Opacity:"),
                    QStringLiteral("optionsArtHistoryOpacity"),
                    controller_ ? paint_->opacity() : 100,
                    [this](int v) { paint_->setOpacity(v); });

    const ArtHistoryOptions initial =
        controller_ ? artHistory_->get() : ArtHistoryOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            ArtHistoryOptions o = artHistory_->get();
            edit(o);
            artHistory_->set(o);
        }
    };
    layout->addWidget(new QLabel(QStringLiteral("Style:"), page));
    auto* style = new QComboBox(page);
    style->setObjectName(QStringLiteral("optionsArtHistoryStyle"));
    for (int i = 0; i < art_history_style_count(); ++i) {
        style->addItem(art_history_style_name(i));
    }
    style->setCurrentIndex(initial.style);
    layout->addWidget(style);
    connect(style, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ArtHistoryOptions& o) { o.style = i; }); });

    auto* area = new NumericField(
        QStringLiteral("Area:"),
        numericConfig(0, 500, 1, 0, QStringLiteral("px"), true,
                      QStringLiteral("optionsArtHistoryArea")),
        page);
    area->setValue(initial.area);
    layout->addWidget(area);
    connect(area, &NumericField::valueChanged, this, [update](double v) {
        update([v](ArtHistoryOptions& o) { o.area = qRound(v); });
    });
    auto* tolerance = new NumericField(
        QStringLiteral("Tolerance:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsArtHistoryTolerance")),
        page);
    tolerance->setValue(initial.tolerance);
    layout->addWidget(tolerance);
    connect(tolerance, &NumericField::valueChanged, this, [update](double v) {
        update([v](ArtHistoryOptions& o) { o.tolerance = qRound(v); });
    });

    layout->addStretch(1);
    return page;
}

// CS6's Eraser bar: the tip, Mode (Brush / Pencil / Block), Opacity, Flow, and
// Erase To History. Block is a fixed square with no Opacity or Flow.
QWidget* OptionsBar::buildEraserPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    const EraserOptions initial = controller_ ? eraser_->get() : EraserOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            EraserOptions o = eraser_->get();
            edit(o);
            eraser_->set(o);
        }
    };
    layout->addWidget(new QLabel(QStringLiteral("Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsEraserMode"));
    mode->addItems({QStringLiteral("Brush"), QStringLiteral("Pencil"), QStringLiteral("Block")});
    mode->setCurrentIndex(initial.mode);
    layout->addWidget(mode);

    NumericField* opacity = addPercentField(
        layout, page, QStringLiteral("Opacity:"), QStringLiteral("optionsEraserOpacity"),
        controller_ ? paint_->opacity() : 100, [this](int v) { paint_->setOpacity(v); });
    NumericField* flow = addPercentField(
        layout, page, QStringLiteral("Flow:"), QStringLiteral("optionsEraserFlow"),
        controller_ ? paint_->flow() : 100, [this](int v) { paint_->setFlow(v); });
    const auto strengthFor = [opacity, flow](int m) {
        opacity->setEnabled(m != 2);
        flow->setEnabled(m != 2);
    };
    strengthFor(initial.mode);
    connect(mode, &QComboBox::currentIndexChanged, this, [update, strengthFor](int i) {
        strengthFor(i);
        update([i](EraserOptions& o) { o.mode = i; });
    });

    auto* toHistory = new QCheckBox(QStringLiteral("Erase to History"), page);
    toHistory->setObjectName(QStringLiteral("optionsEraseToHistory"));
    toHistory->setChecked(initial.toHistory);
    layout->addWidget(toHistory);
    connect(toHistory, &QCheckBox::toggled, this,
            [update](bool on) { update([on](EraserOptions& o) { o.toHistory = on; }); });

    layout->addStretch(1);
    return page;
}

} // namespace pictura
