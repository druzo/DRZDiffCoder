// Objective-C — NSArray with fast enumeration + NSPredicate filter.

#import <Foundation/Foundation.h>

@interface Task : NSObject
@property (nonatomic, copy)   NSString *title;
@property (nonatomic, assign) NSInteger priority;
- (instancetype)initWithTitle:(NSString *)t priority:(NSInteger)p;
@end

@implementation Task
- (instancetype)initWithTitle:(NSString *)t priority:(NSInteger)p {
    if ((self = [super init])) {
        _title = [t copy];
        _priority = p;
    }
    return self;
}
- (NSString *)description {
    return [NSString stringWithFormat:@"%ld  %@", (long)_priority, _title];
}
@end

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSArray *backlog = @[
            [[Task alloc] initWithTitle:@"Write tests"     priority:2],
            [[Task alloc] initWithTitle:@"Fix login bug"   priority:5],
            [[Task alloc] initWithTitle:@"Refactor parser" priority:3],
        ];

        NSPredicate *open = [NSPredicate predicateWithFormat:@"priority >= 3"];
        NSArray *filtered = [backlog filteredArrayUsingPredicate:open];

        for (Task *t in filtered) {
            NSLog(@"%@", t);
        }
    }
    return 0;
}