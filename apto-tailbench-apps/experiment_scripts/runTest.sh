#!/bin/bash

# cargo build --bin main --release

declare -A goals
goals[xapian]=4200000
goals[moses]=2710000
goals[masstree]=427000
goals[silo]=1076000
goals[dnn]=1132000

declare -A targets
targets[xapian]=0.05
targets[moses]=0.05
targets[masstree]=0.05
targets[silo]=0.05
targets[dnn]=0.05

#applications=("xapian" "moses" "masstree" "silo" "dnn")
applications=("silo")

exec_time=120

run_single_triplet_control() {
    app=$1
    app_goal=$2
    dev_target=$3
    outdir=$4

    echo $app $app_goal $dev_target "Control-Based"

    # Multimodule
    #sudo RUST_LOG=info ./target/release/main --exec-time ${exec_time} multimodule $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-multimodule

    # Monolithic
    sudo RUST_LOG=info RUST_BACKTRACE=1 ./target/release/main --exec-time ${exec_time} monolithic $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-monolithic

    # Test non-main
    #sudo RUST_LOG=info RUST_BACKTRACE=1 ./target/release/silo --exec-time ${exec_time} monolithic $app ${app_goal}

    # Dynamic
    # sudo RUST_LOG=info RUST_BACKTRACE=1 ADAPT_TYPE=linear ADAPT_INST=0,1 DEV_TARGET=${dev_target} \
    #      ./target/release/main --exec-time ${exec_time} multimodule $app ${app_goal} > output

    
    #sudo RUST_LOG=info RUST_BACKTRACE=1 ADAPT_TYPE=ewma ALPHA=0.1 ADAPT_INST=0,1 DEV_TARGET=${dev_target} \
    #     ./target/release/main --exec-time ${exec_time} multimodule $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-dynamic
}

run_single_triplet_learning() {
    app=$1
    app_goal=$2
    dev_target=$3
    outdir=$4

    echo $app $app_goal $dev_target "Learning-Based"

    # Multimodule
    #sudo RUST_LOG=info RUST_BACKTRACE=full LEARNING_BASED=y CONF_TYPE=multi \
    #     ./target/release/main --exec-time ${exec_time} multimodule $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-multimodule

    # Monolithic
    sudo RUST_LOG=info RUST_BACKTRACE=1 LEARNING_BASED=y CONF_TYPE=multi \
        ./target/release/main --exec-time ${exec_time} monolithic $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-monolithic

    # Dynamic
    # sudo RUST_LOG=info RUST_BACKTRACE=1 LEARNING_BASED=y CONF_TYPE=multi \
    #      ADAPT_TYPE=linear ADAPT_INST=0,1 DEV_TARGET=${dev_target} \
    #      ./target/release/main --exec-time ${exec_time} multimodule $app ${app_goal} > output
    #python src/fix_output.py output $outdir/$app-dynamic
}

run_single_application() {
    cargo build --release --bin main

    # mkdir -p single-application/control
    # outdir=single-application/control
    # for app in ${applications[@]}; do
    #    app_goal=${goals[$app]}
    #    dev_target=${targets[$app]}

    #    run_single_triplet_control $app $app_goal $dev_target $outdir
    # done

    mkdir -p single-application/learning
    outdir=single-application/learning
    for app in ${applications[@]}; do
        app_goal=${goals[$app]}
        dev_target=${targets[$app]}

        run_single_triplet_learning $app $app_goal $dev_target $outdir
    done
}

run_multi_control() {
    app0=$1
    goal0=$2
    app1=$3
    goal1=$4
    dev_target=$5
    outdir=$6

    echo $app0 $app1 $goal0 $goal1 $dev_target $outdir "Control-Based"

    # sudo RUST_LOG=info ./target/release/main --exec-time 180 multi-app $app0 $goal0 $app1 $goal1 > output
    # python src/fix_output.py output $outdir/${app0}-${app1}-multimodule

    sudo RUST_LOG=info ADAPT_TYPE=linear ADAPT_INST=0,1,2 DEV_TARGET=$dev_target \
         ./target/release/main --exec-time 180 multi-app $app0 $goal0 $app1 $goal1 > output
    # python src/fix_output.py output $outdir/${app0}-${app1}-dynamic
}

run_multi_learning() {
    app0=$1
    goal0=$2
    app1=$3
    goal1=$4
    dev_target=$5
    outdir=$6

    echo $app0 $app1 $goal0 $goal1 $dev_target $outdir "Learning-Based"

    # sudo RUST_LOG=info LEARNING_BASED=y CONF_TYPE=multi \
    #      ./target/release/main --exec-time 180 multi-app $app0 $goal0 $app1 $goal1 > output
    # python src/fix_output.py output $outdir/${app0}-${app1}-multimodule

    sudo RUST_LOG=info LEARNING_BASED=y CONF_TYPE=multi \
         ADAPT_TYPE=linear ADAPT_INST=0,1,2 DEV_TARGET=$dev_target \
         ./target/release/main --exec-time 180 multi-app $app0 $goal0 $app1 $goal1 > output
    # python src/fix_output.py output $outdir/${app0}-${app1}-dynamic
}

run_multi_application() {
    cargo build --release --bin main

    mkdir -p multi-application/control
    mkdir -p multi-application/learning

    last_idx=$((${#applications[@]} - 1))

    for idx0 in $(seq 0 $last_idx); do
        app0=${applications[$idx0]}
        goal0=${goals[$app0]}
        dev_target=${targets[$app0]}

        for idx1 in $(seq $(($idx0 + 1)) $last_idx); do
            app1=${applications[$idx1]}
            goal1=${goals[$app1]}

            run_multi_control $app0 $goal0 $app1 $goal1 $dev_target multi-application/control
            # run_multi_control $app0 $goal0 $app1 $goal1 $dev_target multi-application/learning
        done

    done
}

variant=$1
if [ $variant == "single" ]; then
    run_single_application
elif [ $variant == "multi" ]; then
    run_multi_application
fi
