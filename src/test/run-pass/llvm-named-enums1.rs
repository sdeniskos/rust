enum maybe<T> {
    some(T),
    none
}

type sched_opts = {
    mode: int
};

type task_opts = {
    sched: maybe<sched_opts>,
};

fn default_task_opts() -> task_opts {
    {
        sched: none
    }
}

fn spawn_sched() {
    // This is ok
    let x = {
        sched: none
        with default_task_opts()
    };
}

fn spawn_sched_bad() {
    // This is not
    let x = {
        sched: some({mode: 0})
        with default_task_opts()
    };
}

fn main() {}