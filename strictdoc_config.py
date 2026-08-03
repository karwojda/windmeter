from strictdoc.core.project_config import ProjectConfig


def create_config() -> ProjectConfig:
    return ProjectConfig(
        project_title="Hello World",
        project_features=[
            # Stable features.
            "TABLE_SCREEN",
            "TRACEABILITY_SCREEN",
            "DEEP_TRACEABILITY_SCREEN",
            "SEARCH",
            "TRACEABILITY_MATRIX_SCREEN",
            "REQUIREMENT_TO_SOURCE_TRACEABILITY",
            # "MATHJAX"

            # Experimental features.
            # "PROJECT_STATISTICS_SCREEN",
            # "TREE_MAP_SCREEN",
            # "REQIF",
            "HTML2PDF",
            # "DIFF",
        ],
        include_doc_paths=[
            "/requirements/docs/",
        ],
        include_source_paths=[
            "/src/**",
        ],
    )
