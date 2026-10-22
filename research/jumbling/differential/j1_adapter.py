"""J1 observations for the differential harness; no candidate implementation."""

from dataclasses import replace
import hashlib

import numpy as np
import sim
from sim.twists import Twist, TwistError, TwistMenu

from .harness import EMPTY_MOVED_SET, HarnessInputError, StepResult, moved_set_hash
from .menu_order import order_digest


class _ObservedState(sim.State):
    """Capture the inside set at J1's successful commit, without classifying again."""

    committed_inside = None

    def _commit(self, twist, inside, journal):
        super()._commit(twist, inside, journal)
        self.committed_inside = inside.copy()


class J1Engine:
    def __init__(self, ctx=None, name='J1'):
        self.name = name
        self._ctx = sim.get_context() if ctx is None else ctx
        self._state = None
        self._menu = None
        self._cap_twists = {}
        self._previous = None

    @property
    def state(self):
        """J1 state for test inspection; callers must not mutate it."""
        if self._state is None:
            raise HarnessInputError('J1Engine.start must be called first')
        return self._state

    def _observe(self, status, moved=0, moved_set=EMPTY_MOVED_SET):
        lattice = hashlib.sha256(self.state.lattice_flags().astype(np.uint8).tobytes()).hexdigest()
        self._previous = StepResult(status, moved, moved_set, self.state.digest(), lattice)
        return self._previous

    def _refused(self, status):
        self._previous = replace(self._previous, status=status, moved=0, moved_set=EMPTY_MOVED_SET)
        return self._previous

    def start(self, menu):
        loaded = None
        if menu is not None:
            try:
                loaded = TwistMenu.from_record(self._ctx, menu)
                if len(loaded.items) != len(menu['base']):
                    raise HarnessInputError('J1 dropped duplicate menu items')
                if order_digest(loaded.record()) != order_digest(menu):
                    raise HarnessInputError('J1 changed the menu item order')
            except (TwistError, ValueError, KeyError, TypeError) as exc:
                raise HarnessInputError(f'cannot load J1 menu: {exc}') from exc
        self._state = _ObservedState(self._ctx, menu=loaded)
        self._menu = loaded
        self._cap_twists = {}
        return self._observe('start')

    def apply(self, step):
        state = self.state
        if step['kind'] == 'record':
            try:
                twist = Twist.from_record(self._ctx, step['record'])
            except (TwistError, ValueError):
                return self._refused('invalid')
        elif step['kind'] == 'letter':
            if self._menu is None:
                raise HarnessInputError('a J1 letter requires a started menu')
            grip, item = step['grip'], step['item']
            if type(grip) is not int or not 0 <= grip < 600:
                raise HarnessInputError('letter grip must be an integer in 0..599')
            if type(item) is not int or not 0 <= item < len(self._menu.items):
                raise HarnessInputError('letter item is outside the started menu')
            if grip not in self._cap_twists:
                self._cap_twists[grip] = self._menu.for_cap(grip)
            twist = self._cap_twists[grip][item]
        else:
            raise HarnessInputError('unknown J1 step kind')
        state.committed_inside = None
        outcome = state.apply(twist)
        if outcome.status != 'admissible':
            return self._refused(outcome.status)
        if state.committed_inside is None:
            raise AssertionError('J1 admitted a twist without committing its inside set')
        return self._observe(outcome.status, outcome.moved,
                             moved_set_hash(state.committed_inside.tolist()))

    def survey(self, grips):
        letters = {'admissible': 'a', 'blocked': 'b', 'uncertain': 'u'}
        return {grip: letters[row['status']] for grip, row in self.state.survey(grips).items()}
