"""Source regression for forecast command rejection; no native host is run."""
from pathlib import Path
import re
import unittest


NATIVE = Path(__file__).resolve().parents[1] / 'work/experiments/magic600-04/native'


class ForecastGuardTests(unittest.TestCase):
    def forecast_commands(self, source, method_name, rejection, definitions):
        method = re.search(
            rf'\b{method_name}\(string id\)\s*\{{(.*?)(?=\n \w|\Z)',
            source, re.S,
        )
        self.assertIsNotNone(method, f'{method_name} method not found')
        body = method[1]
        guard = re.search(
            r'if\s*\(\s*hub\.SelectionIsForecast\s*&&\s*'
            r'(new\s*\[\]\s*\{[^}]+\}|\w+)'
            r'\s*\.Contains\s*\(\s*id\s*\)\s*\)\s*' + rejection,
            body, re.S,
        )
        self.assertIsNotNone(guard, f'{method_name} forecast rejection not found')
        if method_name == 'RunCommand':
            self.assertLess(guard.end(), body.index('action();'),
                            'Forecast commands must be rejected before dispatch')
        collection = guard[1]
        if not collection.startswith('new'):
            declaration = re.search(
                r'static\s+readonly\s+string\[\]\s+' + re.escape(collection)
                + r'\s*=\s*new\s*\[\]\s*\{([^}]+)\}\s*;',
                definitions, re.S,
            )
            self.assertIsNotNone(declaration, f'{collection} shared array not found')
            collection = declaration[1]
        commands = set(re.findall(r'"([a-z][a-z-]*)"', collection))
        self.assertTrue(commands, f'{method_name} forecast restriction is empty')
        return commands

    def test_direct_guard_covers_tool_forecast_restrictions(self):
        tools = (NATIVE / 'ExperimentTools.cs').read_text(encoding='utf-8-sig')
        shell = (NATIVE / 'ExperimentShell.cs').read_text(encoding='utf-8-sig')
        definitions = tools + '\n' + shell
        restricted = self.forecast_commands(
            tools, 'ToolCommandUnavailable', r'return\s+"', definitions,
        )
        rejected = self.forecast_commands(
            shell, 'RunCommand', r'throw\s+new\s+InvalidOperationException\s*\(',
            definitions,
        )
        position_commands = {
            'block-capture-position', 'block-protect-position',
            'block-unprotect-exact', 'block-unprotect-position',
        }
        self.assertTrue(position_commands <= restricted,
                        'The four Solve position commands must remain forecast-restricted')
        self.assertFalse(restricted - rejected,
                         f'RunCommand lacks forecast guards: {sorted(restricted - rejected)}')


if __name__ == '__main__':
    unittest.main()
