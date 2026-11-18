#pragma once

#include "api/document_bridge.h"

#include <QtCore/QSaveFile>

namespace pictura {

// Hold the same destination policy for Qt outputs and Rust document writers.
class ProtectedOutput : public QSaveFile {
public:
    using QSaveFile::QSaveFile;

    bool open(QIODevice::OpenMode mode) override
    {
        return !output_path_protected(fileName()) && QSaveFile::open(mode);
    }

    bool commit()
    {
        if (output_path_protected(fileName())) {
            cancelWriting();
            return false;
        }
        return QSaveFile::commit();
    }
};

} // namespace pictura
