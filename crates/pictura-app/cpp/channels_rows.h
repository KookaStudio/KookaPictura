#pragma once

#include <QtCore/QList>
#include <QtCore/QString>

namespace pictura {

struct ChannelRowDef {
    const char* name;
    const char* shortcut;
};

// CS6's rows per mode: the composite on Ctrl+2, the channels from Ctrl+3
// (Multichannel has no composite and starts at Ctrl+1). Shared by the Channels
// panel and the Load Selection dialog so the two colour-row lists cannot drift.
inline QList<ChannelRowDef> channelsForMode(const QString& mode)
{
    if (mode == QLatin1String("grayscale")) {
        return {{"Gray", "Ctrl+2"}};
    }
    if (mode == QLatin1String("cmyk")) {
        return {{"CMYK", "Ctrl+2"},    {"Cyan", "Ctrl+3"},  {"Magenta", "Ctrl+4"},
                {"Yellow", "Ctrl+5"},  {"Black", "Ctrl+6"}};
    }
    if (mode == QLatin1String("lab")) {
        return {{"Lab", "Ctrl+2"}, {"Lightness", "Ctrl+3"}, {"a", "Ctrl+4"}, {"b", "Ctrl+5"}};
    }
    if (mode == QLatin1String("multichannel")) {
        return {{"Cyan", "Ctrl+1"}, {"Magenta", "Ctrl+2"}, {"Yellow", "Ctrl+3"}};
    }
    if (mode == QLatin1String("indexed")) {
        return {{"Index", "Ctrl+2"}};
    }
    if (mode == QLatin1String("bitmap")) {
        return {{"Bitmap", "Ctrl+2"}};
    }
    if (mode == QLatin1String("duotone")) {
        return {{"Duotone", "Ctrl+2"}};
    }
    return {{"RGB", "Ctrl+2"}, {"Red", "Ctrl+3"}, {"Green", "Ctrl+4"}, {"Blue", "Ctrl+5"}};
}

} // namespace pictura
