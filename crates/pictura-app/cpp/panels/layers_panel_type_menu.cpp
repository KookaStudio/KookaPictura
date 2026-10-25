// The CS6 type-only rows of the Layers row menu: orientation, anti-aliasing,
// point/paragraph conversion, and Warp Text.

#include "layers_panel.h"

#include "api/type_api.h"
#include "warp_text_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QPointer>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>

namespace pictura {

namespace {

void addUnimplemented(QMenu& menu, const QString& text)
{
    QAction* action = menu.addAction(text);
    action->setEnabled(false);
    action->setToolTip(QObject::tr("%1 — not implemented yet").arg(text));
}

QString warpRefusalText(type_api::WarpRefusal refusal)
{
    const QString lead
        = QObject::tr("Could not complete the Warp Text command because the type ");
    switch (refusal) {
    case type_api::WarpRefusal::OnPath:
        return lead + QObject::tr("is on a path.");
    case type_api::WarpRefusal::FauxBold:
        return lead + QObject::tr("layer uses a faux bold style.");
    case type_api::WarpRefusal::NoOutlines:
        return lead + QObject::tr("layer uses a font that has no outline data.");
    default:
        return lead + QObject::tr("layer cannot be warped.");
    }
}

} // namespace

void LayersPanel::addTypeRows(QMenu& menu, const QString& path, bool& emitted)
{
    if (!view_ || path.isEmpty() || !type_api::isType(*view_, path)) {
        return;
    }
    if (emitted) {
        menu.addSeparator();
    }
    emitted = true;

    addUnimplemented(menu, tr("Create Work Path"));
    addUnimplemented(menu, tr("Convert to Shape"));
    menu.addSeparator();

    const auto orientation = [this, path](bool vertical) {
        TypeSetting setting = type_api::layerSetting(*view_, path);
        if (setting.vertical == vertical) {
            return;
        }
        setting.vertical = vertical;
        if (type_api::updateLayer(*view_, path, type_api::layerText(*view_, path),
                                  type_api::layerFont(*view_, path), setting,
                                  type_api::layerCharacterSetting(*view_, path),
                                  type_api::layerParagraphSetting(*view_, path))) {
            refresh();
        }
    };
    QAction* horizontal = menu.addAction(tr("Horizontal"));
    horizontal->setCheckable(true);
    horizontal->setChecked(!type_api::layerSetting(*view_, path).vertical);
    connect(horizontal, &QAction::triggered, this, [orientation] { orientation(false); });
    QAction* vertical = menu.addAction(tr("Vertical"));
    vertical->setCheckable(true);
    vertical->setChecked(type_api::layerSetting(*view_, path).vertical);
    connect(vertical, &QAction::triggered, this, [orientation] { orientation(true); });

    menu.addSeparator();
    const QStringList antiAliasNames{tr("None"), tr("Sharp"), tr("Crisp"), tr("Strong"),
                                     tr("Smooth")};
    const int antiAlias = type_api::layerCharacterSetting(*view_, path).anti_alias;
    for (int i = 0; i < int(antiAliasNames.size()); ++i) {
        QAction* action = menu.addAction(antiAliasNames.at(i));
        action->setCheckable(true);
        action->setChecked(antiAlias == i);
        connect(action, &QAction::triggered, this, [this, path, i] {
            CharacterSetting character = type_api::layerCharacterSetting(*view_, path);
            if (character.anti_alias == i) {
                return;
            }
            character.anti_alias = i;
            if (type_api::updateLayer(*view_, path, type_api::layerText(*view_, path),
                                      type_api::layerFont(*view_, path),
                                      type_api::layerSetting(*view_, path), character,
                                      type_api::layerParagraphSetting(*view_, path))) {
                refresh();
            }
        });
    }
    addUnimplemented(menu, tr("Windows LCD"));
    addUnimplemented(menu, tr("Windows"));

    menu.addSeparator();
    const bool paragraph = type_api::isParagraph(*view_, path);
    QAction* convert = menu.addAction(paragraph ? tr("Convert to Point Text")
                                                : tr("Convert to Paragraph Text"));
    connect(convert, &QAction::triggered, this, [this, path, paragraph] {
        if (type_api::convert(*view_, path, !paragraph)) {
            refresh();
        }
    });
    QAction* warp = menu.addAction(tr("Warp Text…"));
    connect(warp, &QAction::triggered, this, [this, path] {
        if (!view_) {
            return;
        }
        const type_api::WarpRefusal refusal = type_api::warpRefusal(*view_, path);
        if (refusal != type_api::WarpRefusal::None) {
            auto* box = new QMessageBox(QMessageBox::Warning, tr("Warp Text"),
                                        warpRefusalText(refusal), QMessageBox::Ok, this);
            box->setObjectName(QStringLiteral("warpTextRefusal"));
            box->setAttribute(Qt::WA_DeleteOnClose);
            box->open();
            return;
        }
        auto* dialog = new WarpTextDialog(type_api::layerWarp(*view_, path), this);
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        const QPointer<PictureView> guard(view_);
        connect(dialog, &WarpTextDialog::changed, this, [this, dialog, guard, path] {
            if (guard && type_api::previewWarp(*guard, path, dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::accepted, this, [this, dialog, guard, path] {
            if (guard && type_api::commitWarp(*guard, path, dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::rejected, this, [this, guard] {
            if (guard && type_api::cancelWarp(*guard)) {
                refresh();
            }
        });
        dialog->open();
    });
}

} // namespace pictura
