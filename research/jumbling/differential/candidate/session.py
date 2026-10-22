"""Optional jumbling session over the experimental store, not a product format.

The injected Engine owns mechanics. Its replay methods return new states, never
mutate a supplied record, and reject any inadmissible or misrecorded step.
``apply_letter`` may mutate only its draft state and returns the exact journal
record, including the moved count. Labels use labels[slot] = home sticker.
No witness is inferred or searched for: words come from the caller or the
retained journal. Every writer is serialized by its caller, as in store.py.
"""

import copy
from pathlib import Path
from typing import Any, Protocol

from differential.candidate import store


class Engine(Protocol):
    """Mechanical and labels-only operations needed by Session.

    Identities pin the model, menu, item order and contract. Records are plain
    store state records; replay_retained/replay_journal start at solved when
    ``start`` is None, otherwise at the supplied exact pose record. Certificate
    checking includes frame agreement and the menu's fixed ν coordinates.
    """

    identities: dict

    def solved(self) -> Any: ...
    def solved_labels(self) -> Any: ...
    def replay_retained(self, generators, *, start=None) -> Any: ...
    def replay_journal(self, records, *, start=None) -> Any: ...
    def apply_letter(self, state, letter) -> dict: ...
    def state_record(self, state) -> dict: ...
    def state_from_record(self, record) -> Any: ...
    def digest(self, state) -> str: ...
    def is_lattice(self, state) -> bool: ...
    def centres_home(self, state) -> bool: ...
    def nu(self, state) -> tuple: ...
    def labels(self, state) -> Any: ...
    def labels_digest(self, labels) -> str: ...
    def label_certificate(self, state) -> dict: ...
    def apply_generators(self, labels, generators) -> Any: ...


class SessionError(ValueError):
    """A refused command; ``kind`` is stable, messages can include evidence."""

    kind = "session_error"


class StaleRevision(SessionError):
    kind = "stale_revision"


class WrongMode(SessionError):
    kind = "wrong_mode"


class OffLattice(SessionError):
    kind = "off_lattice"


class NuNonzero(SessionError):
    kind = "nu_nonzero"


class Unreachable(SessionError):
    kind = "unreachable"


class WordRequired(SessionError):
    kind = "word_required"


class WitnessMismatch(SessionError):
    kind = "witness_mismatch"


class IdentityMismatch(SessionError):
    kind = "identity_mismatch"


class InvalidInput(SessionError):
    kind = "invalid_input"


class CheckpointRequired(SessionError):
    kind = "checkpoint_required"


class TwistRefused(SessionError):
    kind = "twist_refused"


class LabelDataRequired(SessionError):
    kind = "label_data_required"


def _word(values):
    try:
        word = list(values)
    except TypeError as error:
        raise InvalidInput("retained moves need signed generator ids in ±1..±1200") from error
    if any(type(g) is not int or g == 0 or abs(g) > 1200 for g in word):
        raise InvalidInput("retained moves need signed generator ids in ±1..±1200")
    return word


def _digest(value):
    if (not isinstance(value, str) or len(value) != 64
            or any(c not in "0123456789abcdef" for c in value)):
        raise InvalidInput("label import needs a lowercase SHA-256 labels_digest")
    return value


def _path(path):
    result = Path(path).resolve()
    if result.is_relative_to(Path(__file__).resolve().parents[4]):
        raise InvalidInput("session writes need a directory outside the repository")
    return result


class RetainedOnlySession:
    """Minimal 600-cell-Full writer; no pose or jumbling operations are called.

    A checkpoint anchors a chronological signed-generator journal. Label
    checkpoints carry packed labels, while an import of only a digest remains
    opaque until the caller supplies a matching word. ``checkpoints`` retains
    recovery snapshots before reset/import/restore and explicit checkpoints.
    """

    def __init__(self, engine: Engine, path):
        self.engine, self.path = engine, _path(path)
        self.crash_at = None  # Test seam passed unchanged to store.write_session.
        if self.path.exists():
            self.load(self.path)
        else:
            retained = {"model_identity": copy.deepcopy(engine.identities["model"]),
                        "checkpoint": {"kind": "word", "word": []}, "journal": [],
                        "labels_digest": engine.labels_digest(engine.solved_labels()),
                        "origin": "journal", "checkpoints": []}
            self._commit({"format": store.SESSION_FORMAT, "revision": 0, "retained": retained})

    @property
    def revision(self):
        return self._doc["revision"]

    @property
    def document(self):
        return copy.deepcopy(self._doc)

    def _check(self, rev):
        if type(rev) is not int or rev != self.revision:
            raise StaleRevision(f"command revision {rev!r} differs from current revision "
                                f"{self.revision}; issue the action at the current revision")

    def _retained_guard(self, rev, kind):
        self._check(rev)

    def _commit(self, doc, *, command=None, generators=()):
        store.write_session(self.path, doc, crash_at=self.crash_at)
        self._doc = doc

    def load(self, path):
        path = _path(path)
        doc = store.read_session(path)
        self._check_document(doc)
        self.path, self._doc = path, doc
        return self

    _SNAPSHOT_KEYS = frozenset(("model_identity", "checkpoint", "journal", "labels_digest", "origin"))

    def _check_retained(self, retained):
        """One retained part or recovery snapshot: identity, digest, journal, checkpoint, labels."""
        if not isinstance(retained, dict):
            raise InvalidInput("retained session needs a checkpoint and generator journal")
        if retained.get("model_identity") != self.engine.identities["model"]:
            raise IdentityMismatch("retained.model_identity differs from the engine; "
                                   "use a session for the matching full model")
        _digest(retained.get("labels_digest"))
        _word(retained.get("journal"))
        if not isinstance(retained.get("origin"), str):
            raise InvalidInput("retained session needs an origin")
        checkpoint = retained.get("checkpoint")
        if not isinstance(checkpoint, dict) or checkpoint.get("kind") not in ("word", "labels"):
            raise InvalidInput("retained session needs a word or labels checkpoint")
        # Opaque digest-only imports have no labels to replay yet.
        if "labels" in checkpoint or checkpoint["kind"] == "word":
            self._retained_labels(retained)

    def _check_snapshot(self, snapshot):
        if not isinstance(snapshot, dict) or set(snapshot) != self._SNAPSHOT_KEYS:
            raise InvalidInput("a recovery checkpoint needs exactly a model identity, checkpoint, "
                               "journal, labels digest and origin")
        self._check_retained(snapshot)

    def _check_document(self, doc):
        retained = doc["retained"]
        self._check_retained(retained)
        if not isinstance(retained.get("checkpoints"), list):
            raise InvalidInput("retained session needs recovery checkpoints")
        for snapshot in retained["checkpoints"]:
            self._check_snapshot(snapshot)

    def _retained_labels(self, retained=None):
        retained = self._doc["retained"] if retained is None else retained
        checkpoint = retained["checkpoint"]
        if checkpoint["kind"] == "word":
            labels = self.engine.apply_generators(self.engine.solved_labels(),
                                                  _word(checkpoint["word"]))
        elif "labels" in checkpoint:
            labels = store.decode_ids(checkpoint["labels"], len(self.engine.solved_labels()))
        else:
            raise LabelDataRequired("labels are represented only by a digest; provide label data "
                                    "or a retained word before applying retained moves")
        labels = self.engine.apply_generators(labels, _word(retained["journal"]))
        actual = self.engine.labels_digest(labels)
        if actual != retained["labels_digest"]:
            raise WitnessMismatch(f"retained labels_digest: replay {actual}, stored "
                                  f"{retained['labels_digest']}; restore a matching journal")
        return labels

    @staticmethod
    def _snapshot(retained):
        return copy.deepcopy({k: v for k, v in retained.items() if k != "checkpoints"})

    def _update(self, retained, *, command, generators=()):
        doc = self.document
        doc["revision"] += 1
        doc["retained"] = retained
        self._commit(doc, command=command, generators=generators)

    def retained_moves(self, rev, generators):
        self._retained_guard(rev, "retained moves")
        word = _word(generators)
        retained = copy.deepcopy(self._doc["retained"])
        labels = self.engine.apply_generators(self._retained_labels(), word)
        retained["journal"].extend(word)
        retained["labels_digest"] = self.engine.labels_digest(labels)
        self._update(retained, command="moves", generators=word)

    def reset(self, rev):
        self._retained_guard(rev, "reset")
        retained = copy.deepcopy(self._doc["retained"])
        retained["checkpoints"].append(self._snapshot(retained))
        retained.update(checkpoint={"kind": "word", "word": []}, journal=[], origin="reset",
                        labels_digest=self.engine.labels_digest(self.engine.solved_labels()))
        self._update(retained, command="reset")

    def import_labels(self, rev, labels_digest):
        self._retained_guard(rev, "label import")
        digest = _digest(labels_digest)
        retained = copy.deepcopy(self._doc["retained"])
        retained["checkpoints"].append(self._snapshot(retained))
        retained.update(checkpoint={"kind": "labels", "labels_digest": digest}, journal=[],
                        origin="import", labels_digest=digest)
        self._update(retained, command="import")

    def restore(self, rev, index):
        self._retained_guard(rev, "checkpoint restore")
        retained = copy.deepcopy(self._doc["retained"])
        if type(index) is not int or not 0 <= index < len(retained["checkpoints"]):
            raise InvalidInput("checkpoint restore needs an existing checkpoint index")
        snapshot = copy.deepcopy(retained["checkpoints"][index])
        self._check_snapshot(snapshot)
        retained["checkpoints"].append(self._snapshot(retained))
        retained.update(snapshot, origin="restore")
        self._update(retained, command="restore")

    def retained_command(self, rev, kind):
        self._retained_guard(rev, kind)
        if kind not in ("macro", "block", "certificate", "setup_search"):
            raise InvalidInput("unknown retained-workflow action")
        self._update(copy.deepcopy(self._doc["retained"]), command="retained_command")

    def checkpoint(self, rev, *, witness=None):
        self._retained_guard(rev, "retained checkpoint")
        retained = copy.deepcopy(self._doc["retained"])
        if (witness is None and retained["checkpoint"]["kind"] != "word"
                and "labels" not in retained["checkpoint"]):
            raise CheckpointRequired("checkpoint has neither a witness nor a label certificate; "
                                     "a certified checkpoint action is needed")
        if witness is not None:
            word = _word(witness)
            actual = self.engine.labels_digest(
                self.engine.apply_generators(self.engine.solved_labels(), word))
            if actual != retained["labels_digest"]:
                raise WitnessMismatch(f"witness labels_digest: replay {actual}, stored "
                                      f"{retained['labels_digest']}; supply a matching retained word")
        retained["checkpoints"].append(self._snapshot(retained))
        self._update(retained, command="checkpoint")
        return self._snapshot(retained)

    def export(self, rev):
        self._retained_guard(rev, "retained export")
        labels = self._retained_labels()
        self._update(copy.deepcopy(self._doc["retained"]), command="export")
        return labels


class Session(RetainedOnlySession):
    """Jumble is off until explicitly entered; each successful command commits once.

    A label exit always stores its certificate and exact poses, including when
    a supplied witness additionally anchors the retained journal. Reset/import/
    restore invalidate resume even when they happen to preserve the labels.
    """

    def __init__(self, engine: Engine, path, *, identities):
        self.engine = engine
        self.identities = copy.deepcopy(identities)
        self._identities_match(self.identities)
        super().__init__(engine, path)

    def _identities_match(self, identities):
        expected = self.engine.identities
        for field in ("model", "menu", "menu_order", "contract_revision"):
            if identities.get(field) != expected[field]:
                raise IdentityMismatch(f"{field}: session {identities.get(field)!r}, engine "
                                       f"{expected[field]!r}; use matching session identities")

    @property
    def jumbling(self):
        episodes = self._doc.get("jumbling", {}).get("episodes", [])
        return bool(episodes and episodes[-1]["end"] is None)

    @property
    def state(self):
        if not self.jumbling:
            raise WrongMode("Jumble is off; turn Jumble on before reading poses")
        try:
            return self.engine.state_from_record(self._doc["jumbling"]["episodes"][-1]["state"])
        except (ValueError, KeyError, TypeError, IndexError) as error:
            raise Unreachable(f"stored poses are invalid: {error}; "
                              "restore a complete pose record and admissible journal") from error

    def _retained_guard(self, rev, kind):
        self._check(rev)
        if self.jumbling:
            raise WrongMode(f"Jumble is on; turn Jumble off before the retained {kind} action")

    def _on_guard(self, rev, action):
        self._check(rev)
        if not self.jumbling:
            raise WrongMode(f"Jumble is off; turn Jumble on before {action}")

    def _commit(self, doc, *, command=None, generators=()):
        if command is not None and "jumbling" in doc:
            section = doc["jumbling"]
            if command in ("reset", "import", "restore"):
                section["resume"] = None
            elif command == "moves" and section.get("resume") is not None:
                section["resume"]["retained_moves"].extend(generators)
        super()._commit(doc)

    def _check_document(self, doc):
        super()._check_document(doc)
        episodes = doc.get("jumbling", {}).get("episodes", [])
        if episodes:
            self._replay_episode(doc, len(episodes) - 1)

    def _replay_episode(self, doc, index):
        """Prove the entire ancestry, including earlier exits, before trusting it."""
        episode = doc["jumbling"]["episodes"][index]
        self._identities_match(episode["identities"])
        start = episode["start"]
        try:
            if start["kind"] == "word":
                entered = self.engine.replay_retained(_word(start["word"]))
            else:
                previous = start["episode"]
                self._replay_episode(doc, previous)
                exit_record = doc["jumbling"]["episodes"][previous]["end"]["exit_state"]
                entered = self.engine.replay_retained(_word(start["retained_moves"]),
                                                      start=exit_record)
            reached = self.engine.replay_journal(episode["journal"],
                                                 start=self.engine.state_record(entered))
            stored = self.engine.state_from_record(episode["state"])
        except SessionError:
            raise
        except (ValueError, KeyError, TypeError, IndexError) as error:
            raise Unreachable(f"episode {index} journal does not show reachability: {error}; "
                              "restore a complete admissible journal") from error
        actual, expected = self.engine.digest(reached), self.engine.digest(stored)
        if actual != expected:
            raise Unreachable(f"episode {index} digest: journal {actual}, poses {expected}; "
                              "restore a journal that reaches the poses")
        end = episode["end"]
        if end is not None:
            if end["kind"] != "label":
                raise CheckpointRequired("episode exit has no label certificate; "
                                         "restore a certified label checkpoint")
            certificate = self._certificate(reached)
            if certificate != end["certificate"]:
                fields = sorted(set(certificate) | set(end["certificate"]))
                field = next(f for f in fields if certificate.get(f) != end["certificate"].get(f))
                raise Unreachable(f"episode {index} certificate.{field}: replay "
                                  f"{certificate.get(field)!r}, stored "
                                  f"{end['certificate'].get(field)!r}; restore a matching certificate")
            exit_state = self.engine.state_from_record(end["exit_state"])
            if self.engine.digest(exit_state) != actual:
                raise Unreachable(f"episode {index} exit digest differs; restore matching exit poses")
        return reached

    def _certificate(self, state):
        if not self.engine.is_lattice(state):
            raise OffLattice("pieces are off the lattice; twists that bring pieces back are needed")
        if not self.engine.centres_home(state):
            raise Unreachable("a centre is outside its own chamber; restore an admissible journal")
        if any(self.engine.nu(state)):
            raise NuNonzero("ν is not 0; corrections are needed before a label checkpoint")
        certificate = self.engine.label_certificate(state)
        if certificate is None:
            raise CheckpointRequired("checkpoint has neither a witness nor a label certificate; "
                                     "a certified checkpoint action is needed")
        return certificate

    def _checkpoint(self, witness):
        state = self.state
        certificate = self._certificate(state)
        # Checking the journal is mandatory even if a witness or ν = 0 is supplied.
        self._replay_episode(self._doc, len(self._doc["jumbling"]["episodes"]) - 1)
        word = None if witness is None else _word(witness)
        if word is not None:
            expected = self.engine.digest(self.engine.replay_retained(word))
            actual = self.engine.digest(state)
            if expected != actual:
                raise WitnessMismatch(f"witness pose_digest: replay {expected}, state {actual}; "
                                      "supply a matching retained word")
        return state, certificate, word

    def jumble_on(self, rev, *, word=None):
        self._check(rev)
        if self.jumbling:
            raise WrongMode("Jumble is on; finish the episode before another Jumble on action")
        retained = self._doc["retained"]
        resume = self._doc.get("jumbling", {}).get("resume")
        if word is not None:
            word = _word(word)
            state = self.engine.replay_retained(word)
            start = {"kind": "word", "word": word}
        elif resume is not None:
            index = resume["episode"]
            self._replay_episode(self._doc, index)
            exited = self._doc["jumbling"]["episodes"][index]["end"]["exit_state"]
            moves = _word(resume["retained_moves"])
            state = self.engine.replay_retained(moves, start=exited)
            start = {"kind": "exit", "episode": index, "retained_moves": moves}
        elif retained["origin"] == "journal" and retained["checkpoint"]["kind"] == "word":
            word = _word(retained["checkpoint"]["word"]) + _word(retained["journal"])
            state = self.engine.replay_retained(word)
            start = {"kind": "word", "word": word}
        else:
            raise WordRequired("labels alone do not determine poses; supply a retained word from "
                               "solved for Jumble on after an import, reset or restore")
        actual = self.engine.labels_digest(self.engine.labels(state))
        expected = retained["labels_digest"]
        if actual != expected:
            raise WitnessMismatch(f"entry labels_digest: replay {actual}, retained {expected}; "
                                  "supply a matching retained word")
        certificate = self._certificate(state)
        record = dict(self.engine.state_record(state), revision=rev + 1)
        doc = store.cross_on(self._doc, start, self.identities, record)
        doc["jumbling"]["episodes"][-1]["checkpoints"] = [{
            "revision": rev + 1, "certificate": certificate, "witness": word}]
        doc["jumbling"]["resume"] = None
        self._commit(doc)

    def jumble_letters(self, rev, letters):
        self._on_guard(rev, "menu-letter actions")
        draft = self._replay_episode(self._doc, len(self._doc["jumbling"]["episodes"]) - 1)
        try:
            records = [self.engine.apply_letter(draft, letter) for letter in letters]
        except (ValueError, TypeError, IndexError) as error:
            raise TwistRefused("menu word has an invalid, blocked or uncertain letter; "
                               "a fully admissible menu-word action is needed") from error
        doc = self.document
        episode = doc["jumbling"]["episodes"][-1]
        episode["journal"].extend(records)
        episode["state"] = dict(self.engine.state_record(draft), revision=rev + 1)
        doc["revision"] += 1
        self._commit(doc)

    def menu_change(self, rev, identities):
        self._check(rev)
        if self.jumbling:
            raise WrongMode("a jumbling episode is open; finish it before a menu-change action")
        self._identities_match(identities)
        doc = self.document
        doc["revision"] += 1
        self._commit(doc)
        self.identities = copy.deepcopy(identities)

    def checkpoint(self, rev, *, witness=None):
        self._check(rev)
        if not self.jumbling:
            return super().checkpoint(rev, witness=witness)
        _, certificate, word = self._checkpoint(witness)
        checkpoint = {"revision": rev + 1, "certificate": certificate, "witness": word}
        doc = self.document
        doc["jumbling"]["episodes"][-1].setdefault("checkpoints", []).append(checkpoint)
        doc["revision"] += 1
        self._commit(doc)
        return copy.deepcopy(checkpoint)

    def jumble_off(self, rev, *, witness=None):
        self._on_guard(rev, "the Jumble off action")
        state, certificate, word = self._checkpoint(witness)
        labels = self.engine.labels(state)
        digest = self.engine.labels_digest(labels)
        checkpoint = ({"kind": "word", "word": word} if word is not None else
                      {"kind": "labels", "labels": store.encode_ids(int(x) for x in labels),
                       "labels_digest": digest})
        retained = copy.deepcopy(self._doc["retained"])
        retained["checkpoints"].append(self._snapshot(retained))
        retained.update(checkpoint=checkpoint, journal=[], origin="crossing", labels_digest=digest)
        doc = store.cross_off(self._doc, certificate, self.engine.state_record(state), retained)
        if word is not None:
            doc["jumbling"]["episodes"][-1]["end"]["witness"] = word
        doc["jumbling"]["resume"] = {
            "episode": len(doc["jumbling"]["episodes"]) - 1, "retained_moves": []}
        self._commit(doc)
        return labels

    def export(self, rev):
        self._check(rev)
        if not self.jumbling:
            return super().export(rev)
        state, _, _ = self._checkpoint(None)
        labels = self.engine.labels(state)
        self.checkpoint(rev)
        return labels
