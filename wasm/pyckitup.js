import("./pkg")
  .then((pyckitup) => {
    window.pyckitup = pyckitup;
    if (typeof pyckitupLoaded === "function") {
      pyckitupLoaded();
    }
    if (window.pyckitupData) {
      const { entryModule, width, height, frozenModules } = window.pyckitupData;
      pyckitup.start(entryModule, width, height, frozenModules);
    }
  })
  .catch((e) => {
    console.error("error while starting pyckitup", e);
    throw e;
  });
