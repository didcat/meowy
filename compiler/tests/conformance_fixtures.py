from pathlib import PurePosixPath


def source_file(base, name):
    if not isinstance(name, str):
        raise ValueError("fixture source must be a path string")
    path = PurePosixPath(name)
    if (str(path) != name or "\\" in name or ".." in path.parts or
            len(path.parts) < 2 or path.parts[0] != "sources" or path.suffix != ".mwy"):
        raise ValueError(f"noncanonical fixture source: {name}")
    source = base
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f"symlinked fixture source: {name}")
    if not source.is_file():
        raise ValueError(f"missing fixture source: {name}")
    return source


def case_files(case, base):
    base = base.resolve()
    entry = source_file(base, case["source"])
    if not entry.read_text(encoding="utf-8").strip():
        raise ValueError("empty entry source")
    companions = case.get("companions", [])
    if not isinstance(companions, list) or ("companions" in case and not companions):
        raise ValueError("companions must be a nonempty path list")
    files = [(entry, entry.relative_to(entry.parent))]
    seen = {entry}
    for name in companions:
        source = source_file(base, name)
        if not source.is_relative_to(entry.parent):
            raise ValueError(f"companion outside entry directory: {name}")
        if source in seen:
            raise ValueError(f"duplicate fixture source: {name}")
        source.read_text(encoding="utf-8")
        files.append((source, source.relative_to(entry.parent)))
        seen.add(source)
    return files
