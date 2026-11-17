#include "layer_comps_api.h"

#include "layer_comps_bridge.h"

namespace pictura::layer_comps_api {

int count(const PictureView& view)
{
    return layer_comp_count(view);
}

int id(const PictureView& view, int index)
{
    return layer_comp_id(view, index);
}

QString name(const PictureView& view, int index)
{
    return layer_comp_name(view, index);
}

QString comment(const PictureView& view, int index)
{
    return layer_comp_comment(view, index);
}

int record(const PictureView& view, int index)
{
    return layer_comp_record(view, index);
}

bool warning(const PictureView& view, int index)
{
    return layer_comp_warning(view, index);
}

bool lastAvailable(const PictureView& view)
{
    return layer_comp_last_available(view);
}

int create(PictureView& view, const QString& name, const QString& comment, int recordBits)
{
    return layer_comp_create(view, name, comment, recordBits);
}

bool apply(PictureView& view, int id)
{
    return layer_comp_apply(view, id);
}

bool update(PictureView& view, int id)
{
    return layer_comp_update(view, id);
}

bool remove(PictureView& view, int id)
{
    return layer_comp_delete(view, id);
}

bool rename(PictureView& view, int id, const QString& name)
{
    return layer_comp_rename(view, id, name);
}

bool setOptions(PictureView& view, int id, const QString& name, const QString& comment,
                int recordBits)
{
    return layer_comp_set_options(view, id, name, comment, recordBits);
}

int duplicate(PictureView& view, int id)
{
    return layer_comp_duplicate(view, id);
}

bool clearWarning(PictureView& view, int id)
{
    return layer_comp_clear_warning(view, id);
}

bool clearAllWarnings(PictureView& view)
{
    return layer_comp_clear_all_warnings(view);
}

bool restoreLast(PictureView& view)
{
    return layer_comp_restore_last(view);
}

QString exportTargets(const PictureView& view, const QString& prefix)
{
    return layer_comp_export_targets(view, prefix);
}

bool exportFile(const PictureView& view, int id, const QString& format, bool includeIcc,
                const QString& path)
{
    return layer_comp_export_file(view, id, format, includeIcc, path);
}

bool exportPdf(const PictureView& view, const QString& path)
{
    return layer_comp_export_pdf(view, path);
}

} // namespace pictura::layer_comps_api
