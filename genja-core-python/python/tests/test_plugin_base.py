import pytest

from genja.processor import ProcessorPluginBase


def test_processor_base_locks_group_override():
    with pytest.raises(TypeError, match="must not override group"):

        class BadProcessor(ProcessorPluginBase):
            name = "bad"

            @property
            def group(self) -> str:
                return "RunnerPlugin"


def test_processor_base_locks_group_name_override():
    with pytest.raises(TypeError, match="must use group_name"):

        class BadProcessor(ProcessorPluginBase):
            name = "bad"
            group_name = "RunnerPlugin"


def test_processor_base_locks_internal_group_marker_override():
    with pytest.raises(TypeError, match="must use _locked_group_name"):

        class BadProcessor(ProcessorPluginBase):
            name = "bad"
            _locked_group_name = None
