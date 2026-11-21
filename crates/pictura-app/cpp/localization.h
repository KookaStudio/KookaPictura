#pragma once

#include <QtCore/QList>
#include <QtCore/QLocale>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTranslator>

#include <memory>

namespace pictura {

struct UiLanguageOption {
    QString selection;
    QString label;
};

std::unique_ptr<QTranslator> installUiTranslation(
    const QString& selection, const QStringList& systemLanguages = QLocale::system().uiLanguages());

QString uiTranslationsDirectory();
QList<UiLanguageOption> availableUiLanguages();
QString importUiTranslation(const QString& sourcePath, QString* error);

QLocale uiNumericLocale();

// Some widget APIs auto-detect rich text and provide no plain-text setting.
QString plainUiTooltip(const QString& text);

} // namespace pictura
