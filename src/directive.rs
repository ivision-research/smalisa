#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Directive {
    Class,
    Super,
    Implements,
    Source,
    Field,
    EndField,
    Subannotation,
    EndSubannotation,
    Annotation,
    EndAnnotation,
    Enum,
    Method,
    EndMethod,
    Registers,
    Locals,
    ArrayData,
    EndArrayData,
    PackedSwitch,
    EndPackedSwitch,
    SparseSwitch,
    EndSparseSwitch,
    Catch,
    CatchAll,
    Line,
    Param,
    EndParam,
    Local,
    EndLocal,
    RestartLocal,
    Prologue,
    Epilogue,
}

/*
func (d Directive) String() string {
    switch d {
    case DIRECTIVE_CLASS:
        return ".class"
    case DIRECTIVE_SUPER:
        return ".super"
    case DIRECTIVE_IMPLEMENTS:
        return ".implements"
    case DIRECTIVE_SOURCE:
        return ".source"
    case DIRECTIVE_FIELD:
        return ".field"
    case DIRECTIVE_END_FIELD:
        return ".end field"
    case DIRECTIVE_SUBANNOTATION:
        return ".subannotation"
    case DIRECTIVE_END_SUBANNOTATION:
        return ".end subannotation"
    case DIRECTIVE_ANNOTATION:
        return ".annotation"
    case DIRECTIVE_END_ANNOTATION:
        return ".end annotation"
    case DIRECTIVE_ENUM:
        return ".enum"
    case DIRECTIVE_METHOD:
        return ".method"
    case DIRECTIVE_END_METHOD:
        return ".end method"
    case DIRECTIVE_REGISTERS:
        return ".registers"
    case DIRECTIVE_LOCALS:
        return ".locals"
    case DIRECTIVE_ARRAY_DATA:
        return ".array-data"
    case DIRECTIVE_END:
        return ".end array-data"
    case DIRECTIVE_PACKED_SWITCH:
        return ".packed-switch"
    case DIRECTIVE_END_PACKED_SWITCH:
        return ".end packed-switch"
    case DIRECTIVE_SPARSE_SWITCH:
        return ".sparse-switch"
    case DIRECTIVE_END_SPARSE_SWITCH:
        return ".end sparse-switch"
    case DIRECTIVE_CATCH:
        return ".catch"
    case DIRECTIVE_CATCHALL:
        return ".catchall"
    case DIRECTIVE_LINE:
        return ".line"
    case DIRECTIVE_PARAM:
        return ".param"
    case DIRECTIVE_END_PARAM:
        return ".end param"
    case DIRECTIVE_LOCAL:
        return ".local"
    case DIRECTIVE_END_LOCAL:
        return ".end local"
    case DIRECTIVE_RESTART_LOCAL:
        return ".restart local"
    case DIRECTIVE_PROLOGUE:
        return ".prologue"
    case DIRECTIVE_EPILOGUE:
        return ".epilogue"
    default:
        return "UNKNOWN DIRECTIVE"
    }
}
*/
