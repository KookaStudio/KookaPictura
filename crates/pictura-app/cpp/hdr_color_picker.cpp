#include "panels/color_panel.h"

#include "api/engine_bridge.h"
#include "color_picker_dialog.h"
#include "dialogs.h"
#include "panels/color_swatch_widget.h"
#include "panels/swatches_panel.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <limits>
#include <utility>

namespace pictura {
namespace {

using SceneColor = std::array<double, 3>;
constexpr double kMax = double(std::numeric_limits<float>::max());

SceneColor sceneFromColor(const QColor& color)
{
    const QColor rgb = color.toRgb();
    return {rgb.redF(), rgb.greenF(), rgb.blueF()};
}

QColor displayColor(const SceneColor& color)
{
    return QColor::fromRgbF(std::clamp(color[0], 0.0, 1.0),
                           std::clamp(color[1], 0.0, 1.0),
                           std::clamp(color[2], 0.0, 1.0));
}

QColor baseColor(const SceneColor& color)
{
    const double peak = std::max({1.0, color[0], color[1], color[2]});
    return displayColor({color[0] / peak, color[1] / peak, color[2] / peak});
}

class HdrPicker final : public QDialog {
public:
    HdrPicker(SceneColor initial, const QString& title, int method, double exposure, double gamma,
              QWidget* parent)
        : QDialog(parent), scene_(initial), base_(initial), method_(method),
          exposure_(exposure), gamma_(gamma)
    {
        setObjectName(QStringLiteral("hdrPickerDialog"));
        setWindowTitle(title);
        auto* layout = new QVBoxLayout(this);
        auto* samples = new QHBoxLayout();
        for (auto*& swatch : swatches_) {
            swatch = new ColorSwatchWidget(this);
            swatch->setFocusPolicy(Qt::NoFocus);
            swatch->setAttribute(Qt::WA_TransparentForMouseEvents);
            samples->addWidget(swatch);
        }
        layout->addLayout(samples);
        auto* options = new QFormLayout();
        layout->addLayout(options);
        stopSize_ = new QDoubleSpinBox(this);
        stopSize_->setObjectName(QStringLiteral("hdrPickerStopSize"));
        stopSize_->setRange(0.25, 4.0);
        stopSize_->setSingleStep(0.25);
        stopSize_->setValue(1.0);
        options->addRow(tr("Preview Stop Size"), stopSize_);
        relative_ = new QCheckBox(tr("Relative to Document"), this);
        options->addRow(relative_);
        auto* columns = new QHBoxLayout();
        layout->addLayout(columns);
        picker_ = new ColorPicker(this);
        picker_->setOriginalColor(baseColor(initial));
        picker_->setColor(baseColor(initial));
        columns->addWidget(picker_);
        auto* fields = new QFormLayout();
        columns->addLayout(fields);
        static const char* names[] = {"hdrPickerRed", "hdrPickerGreen", "hdrPickerBlue"};
        const QStringList channels = {tr("Red"), tr("Green"), tr("Blue")};
        for (int c = 0; c < 3; ++c) {
            auto* field = new QDoubleSpinBox(this);
            field->setObjectName(QString::fromLatin1(names[c]));
            field->setDecimals(8);
            field->setRange(0.0, kMax);
            field->setSingleStep(0.01);
            field->setValue(scene_[c]);
            field->setKeyboardTracking(false);
            fields->addRow(channels[c], field);
            values_[c] = field;
            connect(field, &QDoubleSpinBox::valueChanged, this, [this, c](double value) {
                if (syncing_) {
                    return;
                }
                scene_[c] = value;
                const double gain = std::exp2(intensity_->value() / 16.0);
                for (int channel = 0; channel < 3; ++channel) {
                    base_[channel] = scene_[channel] / gain;
                }
                picker_->setColor(baseColor(scene_));
                refreshSwatches();
            });
        }
        intensity_ = new QSlider(Qt::Horizontal, this);
        intensity_->setObjectName(QStringLiteral("hdrPickerIntensity"));
        intensity_->setRange(-320, 320);
        fields->addRow(tr("Intensity"), intensity_);
        stops_ = new QLabel(this);
        fields->addRow(stops_);
        connect(intensity_, &QSlider::valueChanged, this, [this] { applyIntensity(); });
        connect(relative_, &QCheckBox::toggled, this, [this] { refreshSwatches(); });
        connect(stopSize_, &QDoubleSpinBox::valueChanged, this, [this] { refreshSwatches(); });
        connect(picker_, &ColorPicker::colorChanged, this, [this](const QColor& color) {
            const double peak = std::max({1.0, base_[0], base_[1], base_[2]});
            base_ = sceneFromColor(color);
            for (auto& channel : base_) {
                channel *= peak;
            }
            applyIntensity();
        });

        auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
        auto* add = buttons->addButton(tr("Add to Swatches"), QDialogButtonBox::ActionRole);
        add->setObjectName(QStringLiteral("hdrPickerAddSwatch"));
        add->setEnabled(ColorPickerDialog::defaultSwatchGrid() != nullptr);
        connect(add, &QPushButton::clicked, this, [this] { addSwatch(); });
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        layout->addWidget(buttons);
        applyIntensity();
    }

    SceneColor scene() const { return scene_; }

private:
    void applyIntensity()
    {
        const double stops = intensity_->value() / 16.0;
        const double gain = std::exp2(stops);
        syncing_ = true;
        for (int c = 0; c < 3; ++c) {
            scene_[c] = std::min(kMax, base_[c] * gain);
            values_[c]->setValue(scene_[c]);
        }
        syncing_ = false;
        stops_->setText(tr("%1 stops").arg(stops));
        refreshSwatches();
    }

    void refreshSwatches()
    {
        const bool relative = relative_->isChecked();
        for (int i = 0; i < int(swatches_.size()); ++i) {
            const double ev = (relative ? exposure_ : 0.0) + (i - 3) * stopSize_->value();
            const auto argb = hdr_swatch_argb(relative ? method_ : 0, ev, relative ? gamma_ : 1.0,
                                             scene_[0], scene_[1], scene_[2]);
            swatches_[i]->setColor(QColor::fromRgba(argb));
            swatches_[i]->setToolTip(tr("%1 EV").arg(ev));
        }
    }

    void addSwatch()
    {
        auto* grid = ColorPickerDialog::defaultSwatchGrid();
        if (!grid) {
            return;
        }
        const auto quantized = [](double value) { return qRound(std::clamp(value, 0.0, 1.0) * 255); };
        const QColor color(quantized(scene_[0]), quantized(scene_[1]), quantized(scene_[2]));
        const bool lossy = std::abs(scene_[0] - color.red() / 255.0) > 1e-7
            || std::abs(scene_[1] - color.green() / 255.0) > 1e-7
            || std::abs(scene_[2] - color.blue() / 255.0) > 1e-7;
        if (lossy && QMessageBox::question(this, tr("Add to Swatches"),
                tr("Swatches use 8-bit RGB. Clip and round this color before adding it?"),
                QMessageBox::Ok | QMessageBox::Cancel, QMessageBox::Cancel) != QMessageBox::Ok) {
            return;
        }
        grid->addSwatch({color, grid->nextSwatchName()});
    }

    SceneColor scene_;
    SceneColor base_;
    int method_;
    double exposure_;
    double gamma_;
    bool syncing_ = false;
    ColorPicker* picker_ = nullptr;
    QCheckBox* relative_ = nullptr;
    QDoubleSpinBox* stopSize_ = nullptr;
    QSlider* intensity_ = nullptr;
    QLabel* stops_ = nullptr;
    std::array<QDoubleSpinBox*, 3> values_{};
    std::array<ColorSwatchWidget*, 7> swatches_{};
};
} // namespace

std::array<double, 3> ColorState::sceneColor(bool foreground) const
{
    const auto& scene = foreground ? hdrForeground_ : hdrBackground_;
    return scene.value_or(sceneFromColor(foreground ? foreground_ : background_));
}

void ColorState::setSceneColor(bool foreground, const std::array<double, 3>& color)
{
    if (std::any_of(color.begin(), color.end(),
                    [](double value) { return !std::isfinite(value) || value < 0.0 || value > kMax; })) {
        return;
    }
    (foreground ? hdrForeground_ : hdrBackground_) = color;
    (foreground ? foreground_ : background_) = displayColor(color);
    if (foreground) {
        emit foregroundChanged(foreground_);
    } else {
        emit backgroundChanged(background_);
    }
}

void ColorState::swapColors()
{
    std::swap(hdrForeground_, hdrBackground_);
    std::swap(foreground_, background_);
    emit foregroundChanged(foreground_);
    emit backgroundChanged(background_);
}

void ColorState::setHdrPickerContext(std::function<PictureView*()> view, std::function<int()> offset)
{
    view_ = std::move(view);
    offset_ = std::move(offset);
}

void ColorState::openColorPicker(bool foreground, QWidget* parent, const QString& title)
{
    setForegroundActive(foreground);
    if (!maskActive_ && view_) {
        if (auto* view = view_()) {
            const auto options = hdr_preview_options(*view);
            if (options.size() == 3) {
                HdrPicker dialog(sceneColor(foreground), title, int(options[0]),
                                 options[1] + (offset_ ? offset_() / 16.0 : 0.0), options[2], parent);
                if (runDialog(dialog, parent) == QDialog::Accepted) {
                    setSceneColor(foreground, dialog.scene());
                }
                return;
            }
        }
    }
    const QColor picked = ColorPickerDialog::getColor(foreground ? foreground_ : background_,
                                                      parent, title);
    if (picked.isValid()) {
        foreground ? setForeground(picked) : setBackground(picked);
    }
}

} // namespace pictura
