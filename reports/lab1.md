# 功能实现

* trace_request = 0, 1 略;
* trace_request = 2 增加一个struct记录syscall信息，由于每个程序要进行隔离统计，这里使用了程序TrapContext的指针地址作为区分的key，由stack_ptr数组保存并转化成index，将func_cnt[index * 512 + syscall_id]加一，更新_idx = index。读取时直接读func_cnt[_idx * 512 + _id]的值。注意每次进入TrapHandler时，插入一个不存在的syscall_id=512用于更新_idx。

# 简答

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。
    * ch2b_bad_address 提示[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it. 向0x00内存写入失败。
    * ch2b_bad_instructions 提示[kernel] IllegalInstruction in application, kernel killed it. 错误的在U态使用sret指令。
    * ch2b_bad_register 提示[kernel] IllegalInstruction in application, kernel killed it. 访问了S态才能访问的寄存器。
    [rustsbi] RustSBI version 0.3.0-alpha.2

2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
    1. L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。
        sp代表KernelStack上保存的U态寄存器信息数组的起始位置。
        __restore用于第一次运行程序时的寄存器初始化以及从S态回到U态的寄存器恢复。
    2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释
        分别处理了sstatus 保存的是处理器状态
        sepc 保存的是异常模式程序计数器寄存器
        sscratch KernelStack栈顶位置
    3. L50-L56：为何跳过了 x2 和 x4？
        x2 保存的是KernelStack栈顶位置，已经赋值给了sscratch
        x4 是线程寄存器目前程序用不到
    4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
        sp指向用户栈，sscratch指向内核栈
    5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
        sret指令，sret可以返回发生trap前的状态在这里是U态
    6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
        sp指向内核栈顶，sscratch指向用户栈顶
    7. 从 U 态进入 S 态是哪一条指令发生的？
        ecall指令


# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。