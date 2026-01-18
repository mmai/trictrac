# Python bindings

## Génération bindings

```sh
# Generate trictrac python lib as a wheel
maturin build -m store/Cargo.toml --release
# Install wheel in local python env
pip install --no-deps --force-reinstall --prefix .devenv/state/venv target/wheels/*.whl
```

## Usage

Pour vérifier l'accès à la lib : lancer le shell interactif `python`

```python
Python 3.13.11 (main, Dec  5 2025, 16:06:33) [GCC 15.2.0] on linux
Type "help", "copyright", "credits" or "license" for more information.
>>> import store
>>> game = store.TricTrac()
>>> game.get_active_player_id()
1
```

### Appels depuis python

`python bot/python/test.py`

## Interfaces

## Entraînement
