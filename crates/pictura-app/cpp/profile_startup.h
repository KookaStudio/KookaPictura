#pragma once

#include <memory>
#include <QtCore/QByteArray>
#include <QtCore/QStringList>

namespace pictura {

class ProfilePresence {
public:
    ~ProfilePresence();

private:
    struct Private;
    explicit ProfilePresence(std::unique_ptr<Private> data);
    std::unique_ptr<Private> data_;

    friend std::unique_ptr<ProfilePresence> prepareProfile(const QStringList&, bool, QString*);
};

std::unique_ptr<ProfilePresence> prepareProfile(const QStringList& arguments, bool interactive,
                                               QString* warning);
bool resetProfile(QString* error);
bool writeProfileFileDurably(const QString& path, const QByteArray& bytes, QString* error);
bool retireProfileFileDurably(const QString& path, QString* error);

} // namespace pictura
