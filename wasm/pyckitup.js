import("./pkg")
  .then(({ start }) => {
    const { width, height, frozenModules } = window.pyckitupData;
    start(width, height, frozenModules);
  })
  .catch((e) => {
    console.error("error while starting pyckitup", e);
    throw e;
  });
