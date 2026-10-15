#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QListWidget>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

AcrPresetsPanel::AcrPresetsPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    auto* list = new QListWidget(this);
    list->addItem(tr("Image Settings"));
    list->addItem(tr("Camera Raw Defaults"));
    list->addItem(tr("Previous Conversion"));
    list->addItem(tr("Custom Settings"));
    list->addItem(tr("Preset Settings"));
    list->addItem(tr("Apply Preset"));
    list->addItem(tr("Clear Imported Settings"));
    list->addItem(tr("Load Settings…"));
    list->addItem(tr("Save Settings…"));
    column->addWidget(list);

    column->addStretch(1);
}

} // namespace pictura
