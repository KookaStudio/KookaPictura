#include "frame_includes.h"

#include "api/engine_bridge.h"

#include <QtCore/QPointer>
#include <QtCore/QSignalBlocker>
#include <QtGui/QMouseEvent>
#include <QtGui/QActionGroup>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QSlider>

namespace pictura {
namespace {
class ExposureSlider final : public QSlider {
public:
    explicit ExposureSlider(QWidget* parent) : QSlider(Qt::Horizontal, parent) {}

protected:
    void mouseDoubleClickEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::LeftButton) {
            setSliderDown(false);
            setValue(0);
            event->accept();
        } else {
            QSlider::mouseDoubleClickEvent(event);
        }
    }
};
} // namespace

void PicturaMainWindow::registerHdrPreviewHandler()
{
    auto* registry = commandRegistry();
    registry->setEnabledProvider(command_ids::ViewHdrPreviewOptions, [this] {
        const auto* view = activeView();
        return view && !pictura::hdr_preview_options(*view).empty();
    });
    registry->setHandler(command_ids::ViewHdrPreviewOptions, [this] { showHdrPreviewOptions(); });
}

void PicturaMainWindow::showHdrPreviewOptions()
{
    QPointer<PictureView> view = activeView();
    if (!view) {
        return;
    }
    const auto options = pictura::hdr_preview_options(*view);
    if (options.size() != 3) {
        return;
    }
    const quint64 generation = view->document_generation();
    QDialog dialog(this);
    dialog.setObjectName(QStringLiteral("hdrPreviewDialog"));
    dialog.setWindowTitle(tr("32-Bit Preview Options"));
    auto* layout = new QFormLayout(&dialog);
    auto* method = new QComboBox(&dialog);
    method->setObjectName(QStringLiteral("hdrPreviewMethod"));
    method->addItems({tr("Exposure And Gamma"), tr("Highlight Compression")});
    method->setCurrentIndex(int(options[0]));
    layout->addRow(tr("Method"), method);
    auto* exposure = new QDoubleSpinBox(&dialog);
    exposure->setObjectName(QStringLiteral("hdrPreviewExposure"));
    exposure->setRange(-20.0, 20.0);
    exposure->setDecimals(2);
    exposure->setSingleStep(0.25);
    exposure->setSuffix(tr(" EV"));
    exposure->setValue(options[1]);
    layout->addRow(tr("Exposure"), exposure);
    auto* gamma = new QDoubleSpinBox(&dialog);
    gamma->setObjectName(QStringLiteral("hdrPreviewGamma"));
    gamma->setRange(0.1, 9.99);
    gamma->setDecimals(2);
    gamma->setSingleStep(0.05);
    gamma->setValue(options[2]);
    layout->addRow(tr("Gamma"), gamma);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addRow(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    const auto apply = [view, generation, method, exposure, gamma](int mode) {
        const bool adjustable = method->currentIndex() == 0;
        exposure->setEnabled(adjustable);
        gamma->setEnabled(adjustable);
        if (view && view->document_generation() == generation) {
            pictura::set_hdr_preview_options(*view, method->currentIndex(), exposure->value(),
                                            gamma->value(), mode);
        }
    };
    connect(method, &QComboBox::currentIndexChanged, &dialog, [apply] { apply(0); });
    connect(exposure, &QDoubleSpinBox::valueChanged, &dialog, [apply] { apply(0); });
    connect(gamma, &QDoubleSpinBox::valueChanged, &dialog, [apply] { apply(0); });
    apply(0);
    apply(runDialog(dialog, this) == QDialog::Accepted ? 1 : 2);
}

void PicturaMainWindow::buildHdrStatus(QStatusBar* bar, QMenu* menu, QActionGroup* group)
{
    hdrExposureAction_ = menu->addAction(tr("32-Bit Exposure"));
    hdrExposureAction_->setCheckable(true);
    hdrExposureAction_->setData(QStringLiteral("hdrExposure"));
    group->addAction(hdrExposureAction_);
    connect(hdrExposureAction_, &QAction::triggered, this, [this] {
        statusReadout_ = QStringLiteral("hdrExposure");
        updateStatus();
    });
    hdrExposureSlider_ = new ExposureSlider(bar);
    hdrExposureSlider_->setObjectName(QStringLiteral("hdrExposureSlider"));
    hdrExposureSlider_->setAccessibleName(tr("32-Bit Exposure"));
    hdrExposureSlider_->setRange(-160, 160);
    hdrExposureSlider_->setMaximumWidth(Theme::scaled(120));
    hdrExposureSlider_->hide();
    connect(hdrExposureSlider_, &QSlider::valueChanged, this, [this](int offset) {
        if (auto* canvas = imageView()) {
            canvas->setHdrExposure(offset);
            hdrExposureSlider_->setToolTip(tr("%1 EV").arg(offset / 16.0));
        }
    });
}

void PicturaMainWindow::updateHdrStatus(bool visible)
{
    if (!hdrExposureSlider_) {
        return;
    }
    const auto* view = activeView();
    auto* canvas = imageView();
    const bool supported = view && !pictura::hdr_preview_options(*view).empty();
    hdrExposureAction_->setEnabled(supported);
    hdrExposureAction_->setChecked(supported && statusReadout_ == QStringLiteral("hdrExposure"));
    const bool show = supported && visible && canvas && hdrExposureAction_->isChecked();
    hdrExposureSlider_->setVisible(show);
    if (show) {
        const QSignalBlocker blocker(hdrExposureSlider_);
        const int offset = canvas->hdrExposure();
        hdrExposureSlider_->setValue(offset);
        hdrExposureSlider_->setToolTip(tr("%1 EV").arg(offset / 16.0));
        sizeLabel_->hide();
    }
}

} // namespace pictura
