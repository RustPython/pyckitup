import("./pkg")
  .then((pyckitup) => {
    if (typeof pyckitupLoaded === "function") {
      pyckitupLoaded();
    }
    const { entryModule, width, height, frozenModules } = window.pyckitupData;
    pyckitup.start(entryModule, width, height, frozenModules);
  })
  .catch((e) => {
    console.error("error while starting pyckitup", e);
    throw e;
  });
