import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import {
  Excalidraw,
  THEME,
  type ExcalidrawInitialDataState,
} from "@excalidraw/excalidraw";

type Board = {
  id: string;
  name: string;
  scene?: PersistedScene | null;
};

type Portfolio = {
  id: string;
  name: string;
  boards: Board[];
};

type ManagerMode = "portfolio" | "board";
type PersistedScene = ExcalidrawInitialDataState;

type NativePointerPressureWindow = Window & {
  __EXCALIDRAW_NATIVE_POINTER_PRESSURE__?: {
    x: number | null;
    y: number | null;
    pressure: number;
    updatedAt: number;
  } | null;
};

const EMPTY_SCENE: PersistedScene = {
  elements: [],
  appState: {
    showWelcomeScreen: false,
  },
  files: {},
};

const normalizeScene = (scene: PersistedScene | null): PersistedScene => {
  const sourceAppState = (scene?.appState ?? {}) as Record<string, unknown>;

  return {
    elements: scene?.elements ?? [],
    files: scene?.files ?? {},
    appState: {
      viewBackgroundColor: sourceAppState.viewBackgroundColor,
      scrollX: sourceAppState.scrollX,
      scrollY: sourceAppState.scrollY,
      zoom: sourceAppState.zoom,
      gridSize: sourceAppState.gridSize,
      gridStep: sourceAppState.gridStep,
      showWelcomeScreen: false,
    } as PersistedScene["appState"],
  };
};

const PortfolioManager = ({
  portfolios,
  selectedBoardId,
  selectedPortfolioId,
  mode,
  error,
  onClose,
  onCreateBoard,
  onCreatePortfolio,
  onSelectBoard,
  onSelectPortfolio,
  onModeChange,
}: {
  portfolios: Portfolio[];
  selectedBoardId: string | null;
  selectedPortfolioId: string | null;
  mode: ManagerMode;
  error: string | null;
  onClose: () => void;
  onCreateBoard: (name: string, portfolioId: string) => Promise<void>;
  onCreatePortfolio: (name: string) => Promise<void>;
  onSelectBoard: (
    portfolioId: string,
    boardId: string,
  ) => void | Promise<void>;
  onSelectPortfolio: (portfolioId: string) => void;
  onModeChange: (mode: ManagerMode) => void;
}) => {
  const [name, setName] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const selectedPortfolio = portfolios.find(
    (portfolio) => portfolio.id === selectedPortfolioId,
  );

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setFormError(null);

    try {
      if (mode === "portfolio") {
        await onCreatePortfolio(name);
      } else if (selectedPortfolioId) {
        await onCreateBoard(name, selectedPortfolioId);
      } else {
        throw new Error("Select a portfolio first");
      }

      setName("");
    } catch (submitError) {
      setFormError(
        submitError instanceof Error
          ? submitError.message
          : "The item could not be saved",
      );
    }
  };

  return (
    <aside className="portfolio-manager" aria-label="Portfolio manager">
      <header className="portfolio-manager__header">
        <div>
          <p className="portfolio-manager__eyebrow">Local workspace</p>
          <h1>Portfolios</h1>
        </div>
        <button
          className="portfolio-manager__close"
          type="button"
          onClick={onClose}
          aria-label="Close portfolio manager"
        >
          ×
        </button>
      </header>

      <div className="portfolio-manager__actions">
        <button
          type="button"
          className={mode === "portfolio" ? "is-active" : ""}
          onClick={() => onModeChange("portfolio")}
        >
          + Portfolio
        </button>
        <button
          type="button"
          className={mode === "board" ? "is-active" : ""}
          onClick={() => onModeChange("board")}
        >
          + Board
        </button>
      </div>

      {error && <p className="portfolio-manager__error">{error}</p>}

      <div className="portfolio-manager__list">
        {portfolios.length === 0 ? (
          <p className="portfolio-manager__empty">
            Create your first portfolio to organize your boards.
          </p>
        ) : (
          portfolios.map((portfolio) => (
            <section className="portfolio-manager__portfolio" key={portfolio.id}>
              <button
                type="button"
                className={`portfolio-manager__portfolio-name ${
                  portfolio.id === selectedPortfolioId ? "is-selected" : ""
                }`}
                onClick={() => onSelectPortfolio(portfolio.id)}
              >
                <span>▾</span>
                {portfolio.name}
              </button>

              <div className="portfolio-manager__boards">
                {portfolio.boards.length === 0 ? (
                  <span className="portfolio-manager__no-boards">
                    No boards yet
                  </span>
                ) : (
                  portfolio.boards.map((board) => (
                    <button
                      type="button"
                      key={board.id}
                      className={
                        board.id === selectedBoardId
                          ? "portfolio-manager__board is-selected"
                          : "portfolio-manager__board"
                      }
                      onClick={() => onSelectBoard(portfolio.id, board.id)}
                    >
                      <span>▱</span>
                      {board.name}
                    </button>
                  ))
                )}
              </div>
            </section>
          ))
        )}
      </div>

      <form className="portfolio-manager__form" onSubmit={submit}>
        <label htmlFor="portfolio-item-name">
          {mode === "portfolio" ? "New portfolio" : "New board"}
        </label>
        <div className="portfolio-manager__form-row">
          <input
            id="portfolio-item-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder={mode === "portfolio" ? "My portfolio" : "Board 1"}
            autoComplete="off"
          />
          <button type="submit">Create</button>
        </div>
        {mode === "board" && !selectedPortfolio && (
          <p className="portfolio-manager__hint">
            Select a portfolio before creating a board.
          </p>
        )}
        {formError && <p className="portfolio-manager__error">{formError}</p>}
      </form>
    </aside>
  );
};

const ExcalidrawApp = () => {
  const [portfolios, setPortfolios] = useState<Portfolio[]>([]);
  const [managerOpen, setManagerOpen] = useState(false);
  const [managerMode, setManagerMode] = useState<ManagerMode>("portfolio");
  const [selectedPortfolioId, setSelectedPortfolioId] = useState<string | null>(
    null,
  );
  const [selectedBoardId, setSelectedBoardId] = useState<string | null>(null);
  const [activeScene, setActiveScene] = useState<PersistedScene>(EMPTY_SCENE);
  const [sceneLoading, setSceneLoading] = useState(false);
  const [theme, setTheme] = useState<typeof THEME.LIGHT | typeof THEME.DARK>(
    THEME.DARK,
  );
  const [error, setError] = useState<string | null>(null);
  const nativePressureTargetRef = useRef<HTMLElement | null>(null);

  const pendingSaveRef = useRef<{
    boardId: string;
    scene: PersistedScene;
  } | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const switchRequestRef = useRef(0);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<Portfolio[]>("list_portfolios");
      setPortfolios(result);
      setError(null);

      if (!selectedPortfolioId && result[0]) {
        setSelectedPortfolioId(result[0].id);
      }
    } catch {
      setError("The portfolio manager requires the desktop application.");
    }
  }, [selectedPortfolioId]);

  const refreshNativeMenu = useCallback(async () => {
    try {
      await invoke("refresh_menu");
    } catch {
      // The browser preview has no native menu; the desktop build does.
    }
  }, []);

  const flushPendingSceneSave = useCallback(async () => {
    if (saveTimerRef.current) {
      clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }

    const pendingSave = pendingSaveRef.current;
    pendingSaveRef.current = null;

    if (!pendingSave) {
      return;
    }

    await invoke("save_board_scene", pendingSave);
  }, []);

  const queueSceneSave = useCallback(
    (boardId: string, scene: PersistedScene) => {
      pendingSaveRef.current = { boardId, scene };

      if (saveTimerRef.current) {
        clearTimeout(saveTimerRef.current);
      }

      saveTimerRef.current = setTimeout(() => {
        saveTimerRef.current = null;
        void flushPendingSceneSave().catch(() => {
          setError("The board could not be saved.");
        });
      }, 500);
    },
    [flushPendingSceneSave],
  );

  const selectBoard = useCallback(
    async (portfolioId: string, boardId: string) => {
      const requestId = ++switchRequestRef.current;

      try {
        await flushPendingSceneSave();
        setSceneLoading(true);
        const scene = await invoke<PersistedScene | null>("get_board_scene", {
          boardId,
        });

        if (requestId !== switchRequestRef.current) {
          return;
        }

        setSelectedPortfolioId(portfolioId);
        setSelectedBoardId(boardId);
        setActiveScene(normalizeScene(scene));
        setError(null);
      } catch {
        if (requestId === switchRequestRef.current) {
          setError("The board could not be opened.");
        }
      } finally {
        if (requestId === switchRequestRef.current) {
          setSceneLoading(false);
        }
      }
    },
    [flushPendingSceneSave],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let active = true;
    let unlistenPressure: (() => void) | undefined;

    void listen<number>("tablet:pressure", ({ payload }) => {
      if (!active) {
        return;
      }

      const ownerWindow =
        nativePressureTargetRef.current?.ownerDocument.defaultView;
      if (!ownerWindow || !Number.isFinite(payload)) {
        return;
      }

      (ownerWindow as NativePointerPressureWindow)
        .__EXCALIDRAW_NATIVE_POINTER_PRESSURE__ = {
        x: null,
        y: null,
        pressure: Math.max(0, Math.min(1, payload)),
        updatedAt: ownerWindow.performance.now(),
      };
    })
      .then((stop) => {
        if (active) {
          unlistenPressure = stop;
        } else {
          stop();
        }
      })
      .catch(() => {
        // The browser preview does not provide Tauri events.
      });

    return () => {
      active = false;
      unlistenPressure?.();
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlistenMenu: (() => void) | undefined;
    let unlistenBoard: (() => void) | undefined;

    void Promise.all([
      listen<string>("portfolio:menu", ({ payload }) => {
        if (!active) {
          return;
        }

        setManagerOpen(true);
        if (payload === "new_portfolio") {
          setManagerMode("portfolio");
        }
        if (payload === "new_board" || payload === "new_board_file") {
          setManagerMode("board");
        }
      }),
      listen<string>("portfolio:open-board", ({ payload }) => {
        if (!active) {
          return;
        }

        setManagerOpen(true);

        const portfolio = portfolios.find((candidate) =>
          candidate.boards.some((board) => board.id === payload),
        );

        if (portfolio) {
          void selectBoard(portfolio.id, payload);
        }
      }),
    ]).then(([menuStop, boardStop]) => {
      if (active) {
        unlistenMenu = menuStop;
        unlistenBoard = boardStop;
      } else {
        menuStop();
        boardStop();
      }
    });

    return () => {
      active = false;
      unlistenMenu?.();
      unlistenBoard?.();
    };
  }, [portfolios, selectBoard]);

  const createPortfolio = async (name: string) => {
    await invoke("create_portfolio", { name });
    await refresh();
    await refreshNativeMenu();
  };

  const createBoard = async (name: string, portfolioId: string) => {
    await flushPendingSceneSave();
    const board = await invoke<Board>("create_board", {
      portfolioId,
      name,
    });
    await refresh();
    await refreshNativeMenu();
    await selectBoard(portfolioId, board.id);
  };

  return (
    <main
      ref={nativePressureTargetRef}
      className={`excalidraw-app ${theme === THEME.DARK ? "is-dark" : ""}`}
    >
      <button
        type="button"
        className="portfolio-manager__toggle"
        onClick={() => setManagerOpen(true)}
      >
        Portfolios
      </button>

      <button
        type="button"
        className="theme-toggle"
        onClick={() =>
          setTheme((currentTheme) =>
            currentTheme === THEME.DARK ? THEME.LIGHT : THEME.DARK,
          )
        }
        aria-label={
          theme === THEME.DARK
            ? "Switch to light mode"
            : "Switch to dark mode"
        }
      >
        {theme === THEME.DARK ? "☀️ Light" : "🌙 Dark"}
      </button>

      {managerOpen && (
        <PortfolioManager
          portfolios={portfolios}
          selectedBoardId={selectedBoardId}
          selectedPortfolioId={selectedPortfolioId}
          mode={managerMode}
          error={error}
          onClose={() => setManagerOpen(false)}
          onCreateBoard={createBoard}
          onCreatePortfolio={createPortfolio}
          onSelectBoard={selectBoard}
          onSelectPortfolio={setSelectedPortfolioId}
          onModeChange={setManagerMode}
        />
      )}

      <Excalidraw
        key={selectedBoardId ?? "unassigned"}
        aiEnabled={false}
        autoFocus
        handleKeyboardGlobally
        theme={theme}
        initialData={activeScene}
        onMount={({ excalidrawAPI }) => {
          // WebKitGTK on Linux currently reports stylus input as a mouse with
          // a fixed pressure of 0.5. Keep variable-width drawing enabled so
          // Excalidraw can use its velocity-based pressure simulation there.
          // Browsers that expose real pressure continue to use it normally.
          excalidrawAPI.updateScene({
            appState: {
              currentItemStrokeVariability: "variable",
            },
          });
        }}
        onThemeChange={(nextTheme) => {
          if (nextTheme !== "system") {
            setTheme(nextTheme);
          }
        }}
        onChange={(elements, appState, files) => {
          if (!selectedBoardId || sceneLoading) {
            return;
          }

          queueSceneSave(selectedBoardId, {
            elements,
            appState: {
              viewBackgroundColor: appState.viewBackgroundColor,
              scrollX: appState.scrollX,
              scrollY: appState.scrollY,
              zoom: appState.zoom,
              gridSize: appState.gridSize,
              gridStep: appState.gridStep,
              showWelcomeScreen: false,
            } as PersistedScene["appState"],
            files,
          });
        }}
      />
    </main>
  );
};

export default ExcalidrawApp;
